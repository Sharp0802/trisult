#![cfg(feature = "alloc")]

use std::collections::HashMap;
use trisult::{
    trisult, ContextStack, ContextStackMut, Default, Diagnosed, Diagnosis, Trisult,
};

/// A custom context stack to keep track of JSON paths (e.g., `$.tags.[1]`).
#[derive(Debug, Clone, Default)]
pub struct JsonPath {
    pub segments: Vec<String>,
}

impl ContextStack for JsonPath {
    type Captured = String;
}

impl ContextStackMut for JsonPath {
    type Segment = String;

    fn capture(&self) -> Self::Captured {
        if self.segments.is_empty() {
            "$".to_string()
        } else {
            format!("$.{}", self.segments.join("."))
        }
    }

    fn push(&mut self, segment: Self::Segment) {
        self.segments.push(segment);
    }

    fn pop(&mut self) {
        self.segments.pop();
    }
}

/// A simplified JSON AST for testing.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    String(String),
    Number(i64),
    Object(HashMap<String, JsonValue>),
    Array(Vec<JsonValue>),
    Null,
}

impl From<&str> for JsonValue {
    fn from(s: &str) -> Self {
        JsonValue::String(s.to_string())
    }
}

impl From<i64> for JsonValue {
    fn from(n: i64) -> Self {
        JsonValue::Number(n)
    }
}

impl<const N: usize> From<[(&str, JsonValue); N]> for JsonValue {
    fn from(arr: [(&str, JsonValue); N]) -> Self {
        let mut map = HashMap::new();
        for (k, v) in arr {
            map.insert(k.to_string(), v);
        }
        JsonValue::Object(map)
    }
}

impl From<Vec<JsonValue>> for JsonValue {
    fn from(arr: Vec<JsonValue>) -> Self {
        JsonValue::Array(arr)
    }
}

impl JsonValue {
    fn type_name(&self) -> &'static str {
        match self {
            Self::String(_) => "string",
            Self::Number(_) => "number",
            Self::Object(_) => "object",
            Self::Array(_) => "array",
            Self::Null => "null",
        }
    }
}

/// Non-fatal issues encountered during validation.
#[derive(Debug, Clone, PartialEq)]
pub enum ValWarn {
    UnrecognizedField(String),
    Deprecated(String),
    Style(String),
}

/// Fatal issues that prevent a field from being successfully parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum ValErr {
    MissingField(String),
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    InvalidFormat(String),
    OutOfBounds {
        min: i64,
        max: i64,
        found: i64,
    },
}

pub type ValResult<T> = Trisult<T, ValWarn, ValErr, String, Default>;

#[trisult]
fn expect_object<'a>(
    value: &'a JsonValue,
    #[context] _path: &mut JsonPath,
) -> ValResult<&'a HashMap<String, JsonValue>> {
    match value {
        JsonValue::Object(obj) => Some(obj),
        _ => {
            error!(ValErr::TypeMismatch {
                expected: "object",
                found: value.type_name()
            });
            None
        }
    }
}

#[trisult]
fn expect_string<'a>(value: &'a JsonValue, #[context] _path: &mut JsonPath) -> ValResult<&'a str> {
    match value {
        JsonValue::String(s) => Some(s.as_str()),
        _ => {
            error!(ValErr::TypeMismatch {
                expected: "string",
                found: value.type_name()
            });
            None
        }
    }
}

#[trisult]
fn expect_number(value: &JsonValue, #[context] _path: &mut JsonPath) -> ValResult<i64> {
    match value {
        JsonValue::Number(n) => Some(*n),
        _ => {
            error!(ValErr::TypeMismatch {
                expected: "number",
                found: value.type_name()
            });
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct UserProfile {
    pub username: String,
    pub age: i64,
    pub tags: Vec<String>,
}

#[trisult(segment = "username".to_string())]
fn parse_username(
    obj: &HashMap<String, JsonValue>,
    #[context] path: &mut JsonPath,
) -> ValResult<String> {
    if let Some(val) = obj.get("username") {
        let s = tri!(expect_string(val, path))?;
        if s.len() < 3 {
            error!(ValErr::InvalidFormat(
                "username must be at least 3 characters".to_string()
            ));
            return None;
        }
        if s.chars().next().map(|c| c.is_uppercase()) == Some(true) {
            warn!(ValWarn::Style(
                "usernames are typically lowercase".to_string()
            ));
        }
        Some(s.to_string())
    } else {
        error!(ValErr::MissingField("username".to_string()));
        None
    }
}

#[trisult(segment = "age".to_string())]
fn parse_age(obj: &HashMap<String, JsonValue>, #[context] path: &mut JsonPath) -> ValResult<i64> {
    if let Some(val) = obj.get("age") {
        let n = tri!(expect_number(val, path))?;
        if !(0..=150).contains(&n) {
            error!(ValErr::OutOfBounds {
                min: 0,
                max: 150,
                found: n
            });
            return None;
        }
        Some(n)
    } else {
        error!(ValErr::MissingField("age".to_string()));
        None
    }
}

#[trisult(segment = "tags".to_string())]
fn parse_tags(
    obj: &HashMap<String, JsonValue>,
    #[context] path: &mut JsonPath,
) -> ValResult<Vec<String>> {
    let mut parsed_tags = Vec::new();

    if let Some(val) = obj.get("tags") {
        match val {
            JsonValue::Array(arr) => {
                for (i, item) in arr.iter().enumerate() {
                    path.push(format!("[{i}]"));

                    // Using `tri!` without `?` allows us to accumulate diagnostics across the loop
                    // without short-circuiting the entire array validation upon encountering the first error.
                    if let Some(s) = tri!(expect_string(item, path)) {
                        if s.is_empty() {
                            warn!(ValWarn::Style("empty tags are discouraged".to_string()));
                        }
                        parsed_tags.push(s.to_string());
                    }

                    path.pop();
                }
            }
            _ => {
                error!(ValErr::TypeMismatch {
                    expected: "array",
                    found: val.type_name()
                });
                return None;
            }
        }
        Some(parsed_tags)
    } else {
        Some(vec![])
    }
}

#[trisult]
fn parse_user_profile(value: &JsonValue, #[context] path: &mut JsonPath) -> ValResult<UserProfile> {
    let obj = tri!(expect_object(value, path))?;

    for key in obj.keys() {
        if !["username", "age", "tags", "legacy_id"].contains(&key.as_str()) {
            warn!(ValWarn::UnrecognizedField(key.clone()));
        }
    }

    if obj.contains_key("legacy_id") {
        warn!(ValWarn::Deprecated(
            "legacy_id is no longer supported".to_string()
        ));
    }

    // Evaluate all fields, accumulating diagnostics instead of short-circuiting.
    let username_opt = tri!(parse_username(obj, path));
    let age_opt = tri!(parse_age(obj, path));
    let tags_opt = tri!(parse_tags(obj, path));

    if let (Some(username), Some(age), Some(tags)) = (username_opt, age_opt, tags_opt) {
        Some(UserProfile {
            username,
            age,
            tags,
        })
    } else {
        None
    }
}

#[test]
fn test_valid_profile() {
    let mut path = JsonPath::default();
    let json = JsonValue::from([
        ("username", JsonValue::from("john_doe")),
        ("age", JsonValue::from(30)),
        (
            "tags",
            JsonValue::from(vec![JsonValue::from("rust"), JsonValue::from("developer")]),
        ),
    ]);

    let res = parse_user_profile(&json, &mut path);
    assert!(res.is_ok());
    let Diagnosed(profile, diags) = res.unwrap();
    assert_eq!(profile.username, "john_doe");
    assert_eq!(profile.age, 30);
    assert_eq!(profile.tags, vec!["rust", "developer"]);
    assert!(diags.is_empty());
}

#[test]
fn test_profile_with_warnings() {
    let mut path = JsonPath::default();
    let json = JsonValue::from([
        ("username", JsonValue::from("John_doe")), // Style warning (uppercase)
        ("age", JsonValue::from(30)),
        ("legacy_id", JsonValue::from(12345)), // Deprecated warning
        ("unknown_field", JsonValue::from("ignored")), // Unrecognized warning
        (
            "tags",
            JsonValue::from(vec![JsonValue::from("rust"), JsonValue::from("")]), // Style warning (empty tag)
        ),
    ]);

    let res = parse_user_profile(&json, &mut path);
    assert!(res.is_ok());
    let Diagnosed(profile, warnings) = res.unwrap();

    assert_eq!(profile.username, "John_doe");

    let w_vec: Vec<_> = warnings.into_iter().collect();
    assert_eq!(w_vec.len(), 4);

    assert!(
        w_vec
            .iter()
            .any(|w| matches!(w.value, ValWarn::Style(_)) && w.context == "$.username")
    );
    assert!(
        w_vec
            .iter()
            .any(|w| matches!(w.value, ValWarn::Deprecated(_)) && w.context == "$")
    );
    assert!(
        w_vec
            .iter()
            .any(|w| matches!(w.value, ValWarn::UnrecognizedField(_)) && w.context == "$")
    );
    assert!(
        w_vec
            .iter()
            .any(|w| matches!(w.value, ValWarn::Style(_)) && w.context == "$.tags.[1]")
    );
}

#[test]
fn test_profile_with_multiple_errors() {
    let mut path = JsonPath::default();
    let json = JsonValue::from([
        // username is missing
        ("age", JsonValue::from(-5)), // OutOfBounds error
        (
            "tags",
            JsonValue::from(vec![
                JsonValue::from("rust"),
                JsonValue::from(42), // TypeMismatch error inside array
            ]),
        ),
    ]);

    let res = parse_user_profile(&json, &mut path);
    assert!(res.is_err());
    let errors = res.err().unwrap();

    let e_vec: Vec<_> = errors.into_iter().collect();

    assert_eq!(e_vec.len(), 3);

    assert!(e_vec.iter().any(
        |e| matches!(e.value, Diagnosis::Error(ValErr::MissingField(_)))
            && e.context == "$.username"
    ));
    assert!(e_vec.iter().any(
        |e| matches!(e.value, Diagnosis::Error(ValErr::OutOfBounds { .. })) && e.context == "$.age"
    ));
    assert!(e_vec.iter().any(
        |e| matches!(e.value, Diagnosis::Error(ValErr::TypeMismatch { .. }))
            && e.context == "$.tags.[1]"
    ));
}

#[test]
fn test_mixed_errors_and_warnings_collected() {
    let mut path = JsonPath::default();
    let json = JsonValue::from([
        ("username", JsonValue::from("John_doe")), // Warning
        // age missing -> Error
        ("legacy_id", JsonValue::from(1)), // Warning
    ]);

    let res = parse_user_profile(&json, &mut path);
    assert!(res.is_err());
    let diags = res.err().unwrap();
    let diag_vec: Vec<_> = diags.into_iter().collect();

    assert_eq!(diag_vec.len(), 3);
    assert!(
        diag_vec
            .iter()
            .any(|d| matches!(d.value, Diagnosis::Warning(ValWarn::Style(_))))
    );
    assert!(
        diag_vec
            .iter()
            .any(|d| matches!(d.value, Diagnosis::Warning(ValWarn::Deprecated(_))))
    );
    assert!(
        diag_vec
            .iter()
            .any(|d| matches!(d.value, Diagnosis::Error(ValErr::MissingField(_))))
    );
}

#[test]
fn test_not_an_object() {
    let mut path = JsonPath::default();
    let json = JsonValue::from("just a string");

    let res = parse_user_profile(&json, &mut path);
    assert!(res.is_err());
    let e_vec: Vec<_> = res.err().unwrap().into_iter().collect();

    assert_eq!(e_vec.len(), 1);
    assert!(matches!(
        e_vec[0].value,
        Diagnosis::Error(ValErr::TypeMismatch { .. })
    ));
    assert_eq!(e_vec[0].context, "$");
}
