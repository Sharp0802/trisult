use std::error::Error;
use std::fmt;
use trisult::{AccumulatorKind, Contextual, Contextuals, Diagnosed, Diagnosis, Trisult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineSpan(pub usize);

impl fmt::Display for LineSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Line {}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigWarn {
    Deprecated(String),
    Unconventional(String),
}

impl fmt::Display for ConfigWarn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Deprecated(msg) => write!(f, "Deprecated: {}", msg),
            Self::Unconventional(msg) => write!(f, "Style: {}", msg),
        }
    }
}

impl Error for ConfigWarn {}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigErr {
    MissingField(&'static str),
    InvalidFormat(String),
}

impl fmt::Display for ConfigErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
        }
    }
}
impl Error for ConfigErr {}

type ConfigResult<T> = Trisult<T, ConfigWarn, ConfigErr, LineSpan>;

fn ok_with_warn<T>(value: T, warn: ConfigWarn, line: usize) -> ConfigResult<T> {
    let mut diags = Contextuals::new(AccumulatorKind::All);
    diags.push_naive(Contextual::new(LineSpan(line), warn));
    Trisult::Ok(Diagnosed(value, diags))
}

fn fail<T>(err: ConfigErr, line: usize) -> ConfigResult<T> {
    let mut diags = Contextuals::new(AccumulatorKind::All);
    diags.push(Contextual::new(LineSpan(line), Diagnosis::Error(err)));
    Trisult::Err(diags)
}

fn pure_ok<T>(value: T) -> ConfigResult<T> {
    Trisult::Ok(Diagnosed(value, Contextuals::new(AccumulatorKind::All)))
}

#[derive(Debug, Clone, PartialEq)]
struct ParsedConfig {
    version: i32,
    name: String,
}

fn parse_version(version_str: &str, line: usize) -> ConfigResult<i32> {
    match version_str {
        "v1" => ok_with_warn(
            1,
            ConfigWarn::Deprecated("v1 is deprecated, use v2".into()),
            line,
        ),
        "v2" => pure_ok(2),
        _ => fail(
            ConfigErr::InvalidFormat(format!("Unknown version: {}", version_str)),
            line,
        ),
    }
}

fn parse_name(name_str: &str, line: usize) -> ConfigResult<String> {
    if name_str.is_empty() {
        fail(ConfigErr::MissingField("name"), line)
    } else if name_str.chars().next().unwrap().is_lowercase() {
        ok_with_warn(
            name_str.to_string(),
            ConfigWarn::Unconventional("Names should be capitalized".into()),
            line,
        )
    } else {
        pure_ok(name_str.to_string())
    }
}

fn parse_config(
    version: &str,
    v_line: usize,
    name: &str,
    n_line: usize,
) -> ConfigResult<ParsedConfig> {
    parse_version(version, v_line).and_then(|v| {
        parse_name(name, n_line).map(|n| ParsedConfig {
            version: v,
            name: n,
        })
    })
}

#[test]
fn perfect_config() {
    match parse_config("v2", 1, "Alice", 2) {
        Trisult::Ok(Diagnosed(val, diags)) => {
            // Strictly check the values
            assert_eq!(val.version, 2);
            assert_eq!(val.name, "Alice");

            // Strictly verify no warnings/diagnoses were accumulated
            assert!(diags.is_empty(), "Expected no warnings for a perfect config");
        }
        Trisult::Err(_) => panic!("Expected Trisult::Ok, but got Trisult::Err"),
    }
}

#[test]
fn config_with_warnings() {
    match parse_config("v1", 5, "bob", 6) {
        Trisult::Ok(Diagnosed(val, diags)) => {
            // Verify the successful parse yielded the correct fallback/modified values
            assert_eq!(val.version, 1);
            assert_eq!(val.name, "bob");

            let accumulated_warnings: Vec<_> = diags.iter().collect();
            assert_eq!(accumulated_warnings.len(), 2, "Expected exactly 2 warnings");

            // Check the first warning (Deprecated 'v1')
            assert_eq!(accumulated_warnings[0].context.0, 5);
            assert!(matches!(
                accumulated_warnings[0].value,
                ConfigWarn::Deprecated(_)
            ));

            // Check the second warning (Unconventional lowercase name)
            assert_eq!(accumulated_warnings[1].context.0, 6);
            assert!(matches!(
                accumulated_warnings[1].value,
                ConfigWarn::Unconventional(_)
            ));
        }
        Trisult::Err(_) => panic!("Expected Trisult::Ok, but got Trisult::Err"),
    }
}

#[test]
fn config_with_error() {
    match parse_config("v1", 10, "", 11) {
        Trisult::Ok(_) => panic!("Expected Trisult::Err, but got Trisult::Ok"),
        Trisult::Err(diags) => {
            let accumulated_diags: Vec<_> = diags.iter().collect();

            // Should contain BOTH the warning from parse_version AND the error from parse_name
            assert_eq!(accumulated_diags.len(), 2, "Expected exactly 2 diagnoses (1 warning, 1 error)");

            // Verify the warning was properly preserved and cast to a Diagnosis::Warning
            assert_eq!(accumulated_diags[0].context.0, 10);
            assert!(matches!(
                accumulated_diags[0].value,
                Diagnosis::Warning(ConfigWarn::Deprecated(_))
            ));

            // Verify the error was appended as a Diagnosis::Error
            assert_eq!(accumulated_diags[1].context.0, 11);
            assert!(matches!(
                accumulated_diags[1].value,
                Diagnosis::Error(ConfigErr::MissingField("name"))
            ));
        }
    }
}
