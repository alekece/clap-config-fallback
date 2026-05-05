use std::io::Write;

use clap::{Args, Parser};
use clap_config_fallback::{ConfigArgs, ConfigParser};
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[command(flatten)]
    empty: EmptyArgs,
    #[command(flatten)]
    logging: LoggingArgs,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[derive(Debug, Args, ConfigArgs, PartialEq, Eq)]
struct LoggingArgs {
    #[arg(long)]
    level: String,
    #[arg(long)]
    interval_secs: u16,
}

#[derive(Debug, Args, ConfigArgs, PartialEq, Eq)]
struct EmptyArgs {}

#[test]
fn nested_fields_can_be_loaded_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"profile = "prod""#)?;
    writeln!(file, r#"level = "warn""#)?;
    writeln!(file, "interval_secs = 30")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.profile, "prod");
    assert_eq!(cli.logging.level, "warn");
    assert_eq!(cli.logging.interval_secs, 30);

    Ok(())
}

#[test]
fn nested_cli_values_override_nested_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"profile = "prod""#)?;
    writeln!(file, r#"level = "warn""#)?;
    writeln!(file, "interval_secs = 30")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--profile",
        "dev",
        "--level",
        "error",
        "--interval-secs",
        "10",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.profile, "dev");
    assert_eq!(cli.logging.level, "error");
    assert_eq!(cli.logging.interval_secs, 10);

    Ok(())
}
