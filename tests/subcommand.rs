use std::{io::Write, time::Duration};

use assert_matches::assert_matches;
use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    #[config(path, format = "toml")]
    config_path: String,
    #[command(subcommand)]
    #[config(aliases = ["cmd", "action"])]
    command: Command,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
#[config(tag = "ref")]
enum Command {
    Run(()),
    Build(BuildCommand),
    Debug {
        #[arg(long)]
        verbose: bool,
        #[arg(short, long, value_parser = humantime::parse_duration)]
        #[config(value_format = humantime::format_duration(timeout).to_string())]
        timeout: Option<Duration>,
    },
    #[config(skip)]
    Test,
}

#[derive(Debug, Args, ConfigArgs)]
struct BuildCommand {
    #[arg(long)]
    target: String,
}

#[test]
fn empty_variant_is_loaded_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"ref = "run""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, Command::Run(()));

    Ok(())
}

#[test]
fn newtype_variant_is_loaded_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[cmd]"#)?;
    writeln!(file, r#"ref = "build""#)?;
    writeln!(file, r#"target = "x86_64-unknown-linux-gnu""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, Command::Build(command) if command.target == "x86_64-unknown-linux-gnu");

    Ok(())
}

#[test]
fn struct_variant_is_loaded_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[action]"#)?;
    writeln!(file, r#"ref = "debug""#)?;
    writeln!(file, r#"verbose = true"#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(
        cli.command,
        Command::Debug {
            verbose: true,
            timeout: None
        }
    );

    Ok(())
}

#[test]
fn cli_subcommand_overrides_config_subcommand() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"ref = "build""#)?;
    writeln!(file, r#"target = "x86_64-unknown-linux-gnu""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
        "run",
    ])?;

    assert_matches!(cli.command, Command::Run(()));

    Ok(())
}

#[test]
fn missing_config_tag_returns_error() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"target = "x86_64-unknown-linux-gnu""#)?;

    let result = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

#[test]
fn unknown_config_tag_returns_error() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"ref = "deploy""#)?;

    let result = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

#[test]
fn skipped_config_variant_returns_error() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"ref = "test""#)?;

    let result = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}
