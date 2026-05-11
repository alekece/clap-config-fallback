use std::{fmt, io::Write, str::FromStr};

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct UpperString(String);

impl FromStr for UpperString {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UpperString(s.to_uppercase()))
    }
}

impl fmt::Display for UpperString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

fn to_uppercase(s: &str) -> Result<String, String> {
    Ok(s.to_uppercase())
}

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    positional: Vec<String>,
    #[arg(short, long, value_delimiter = ',', value_parser = to_uppercase)]
    comma_separated: Vec<String>,
    #[arg(short, long)]
    optional: Option<Vec<UpperString>>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn positional_vec_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "a", "b", "c"])?;

    assert_eq!(cli.positional.as_slice(), ["a", "b", "c"]);

    Ok(())
}

#[test]
fn vec_arg_with_value_delimiter_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "-c", "a,b,c"])?;

    assert_eq!(cli.comma_separated.as_slice(), ["A", "B", "C"]);

    Ok(())
}

#[test]
fn repeated_vec_arg_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "-c", "a,b", "-c", "c"])?;

    assert_eq!(cli.comma_separated.as_slice(), ["A", "B", "C"]);

    Ok(())
}

#[test]
fn optional_vec_arg_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "-o", "a", "-o", "b"])?;

    assert_eq!(
        cli.optional.as_ref().unwrap().as_slice(),
        [UpperString("A".to_string()), UpperString("B".to_string())]
    );

    Ok(())
}

#[test]
fn vec_is_parsed_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"positional = ["a", "b", "c"]"#)?;
    writeln!(file, r#"comma_separated = ["a", "b", "c"]"#)?;
    writeln!(file, r#"optional = ["a", "b"]"#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.positional.as_slice(), ["a", "b", "c"]);
    assert_eq!(cli.comma_separated.as_slice(), ["A", "B", "C"]);
    assert_eq!(
        cli.optional.as_ref().unwrap().as_slice(),
        [UpperString("A".to_string()), UpperString("B".to_string())]
    );

    Ok(())
}
