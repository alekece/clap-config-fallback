use std::io::Write;

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[arg(long)]
    debug: bool,
    #[arg(short, long)]
    verbose: Option<bool>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn bool_flag_can_be_enabled_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "debug = true")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert!(cli.debug);

    Ok(())
}

#[test]
fn bool_flag_from_cli_overrides_false_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "debug = false")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--debug",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert!(cli.debug);

    Ok(())
}

#[test]
fn bool_flag_missing_from_cli_and_config_falls_back_to_false() -> Result<()> {
    let file = NamedTempFile::new()?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert!(!cli.debug);

    Ok(())
}

#[test]
fn optional_bool_can_be_enabled_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "verbose = true")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.verbose, Some(true));

    Ok(())
}

#[test]
fn optional_bool_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "--verbose", "true"])?;

    assert_eq!(cli.verbose, Some(true));

    Ok(())
}
