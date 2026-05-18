mod shared;

use crate::shared::span::{Offset, Span};
use std::ops::Deref;
use thiserror::Error;
use trisult::{trisult, Acc, Default, Diagnosed, Diagnosis, Trisult};

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigWarn {
    #[error("deprecated: {0}")]
    Deprecated(&'static str),
    #[error("style: {0}")]
    Unconventional(&'static str),
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ConfigErr {
    #[error("missing field: {0}")]
    MissingField(&'static str),
    #[error("invalid format: {0}")]
    InvalidFormat(&'static str),
}

pub type ConfigResult<T> = Trisult<
    T,
    ConfigWarn,
    ConfigErr,
    <Default as Acc>::Acc<Diagnosis<ConfigWarn, ConfigErr>, Offset>,
>;

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    version: i32,
    name: String,
}

#[trisult]
fn parse_version(version: Span) -> ConfigResult<i32> {
    match version.deref() {
        "v2" => Some(2),
        "v1" => {
            warn!(ConfigWarn::Deprecated("v1 is legacy"), version.at());
            Some(1)
        }
        _ => {
            error!(ConfigErr::InvalidFormat("unknown version"), version.at());
            None
        }
    }
}

#[trisult]
fn parse_name(name: Span) -> ConfigResult<String> {
    if name.is_empty() {
        error!(ConfigErr::MissingField("name"), name.at());
        return None;
    }

    if name.chars().next().unwrap().is_lowercase() {
        warn!(
            ConfigWarn::Unconventional("Names should be capitalized"),
            name.at()
        );
    }

    Some(name.to_string())
}

#[trisult]
fn parse_config(version: Span, name: Span) -> ConfigResult<Config> {
    let version = tri!(parse_version(version))?;
    let name = tri!(parse_name(name))?;
    Some(Config { version, name })
}

#[test]
fn test_parse_config() {
    match parse_config(Span::new("v2", 1), Span::new("Alice", 2)) {
        Trisult::Ok(Diagnosed(val, diags)) => {
            assert_eq!(val.version, 2);
            assert_eq!(val.name, "Alice");
            assert!(diags.is_empty());
        }
        Trisult::Err(_) => panic!("Expected Trisult::Ok, but got Trisult::Err"),
    }
}

#[test]
fn test_parse_config_with_warnings() {
    match parse_config(Span::new("v1", 5), Span::new("bob", 6)) {
        Trisult::Ok(Diagnosed(val, diags)) => {
            assert_eq!(val.version, 1);
            assert_eq!(val.name, "bob");

            let accumulated_warnings: Vec<_> = diags.iter().collect();
            assert_eq!(accumulated_warnings.len(), 2);

            assert_eq!(accumulated_warnings[0].context.0, 5);
            assert!(matches!(
                accumulated_warnings[0].value,
                ConfigWarn::Deprecated(_)
            ));

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
fn test_parse_config_with_error() {
    match parse_config(Span::new("v1", 10), Span::new("", 11)) {
        Trisult::Ok(_) => panic!("Expected Trisult::Err, but got Trisult::Ok"),
        Trisult::Err(diags) => {
            let accumulated_diags: Vec<_> = diags.iter().collect();

            assert_eq!(accumulated_diags.len(), 2);

            assert_eq!(accumulated_diags[0].context.0, 10);
            assert!(matches!(
                accumulated_diags[0].value,
                Diagnosis::Warning(ConfigWarn::Deprecated(_))
            ));

            assert_eq!(accumulated_diags[1].context.0, 11);
            assert!(matches!(
                accumulated_diags[1].value,
                Diagnosis::Error(ConfigErr::MissingField("name"))
            ));
        }
    }
}
