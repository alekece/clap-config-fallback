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
enum Command {
    Run(()),
    Build(BuildCommand),
    Debug {
        #[arg(long)]
        verbose: bool,
        #[arg(short, long, value_parser = humantime::parse_duration)]
        #[config(value_format = humantime::format_duration)]
        timeout: Option<Duration>,
    },
    Empty {},
    #[config(skip)]
    Test,
}

#[derive(Debug, Args, ConfigArgs)]
struct BuildCommand {
    #[arg(long)]
    target: String,
}

#[test]
fn unit_variant_in_config_without_cli_command_returns_error() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"run = true"#)?;

    let result = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

#[test]
fn newtype_variant_config_provides_fallback_for_cli_command() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[cmd.build]"#)?;
    writeln!(file, r#"target = "x86_64-unknown-linux-gnu""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
        "build",
    ])?;

    assert_matches!(cli.command, Command::Build(ref cmd) if cmd.target == "x86_64-unknown-linux-gnu");

    Ok(())
}

#[test]
fn struct_variant_config_provides_fallback_for_cli_command() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[action.debug]"#)?;
    writeln!(file, r#"verbose = true"#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
        "debug",
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

    writeln!(file, r#"[command.build]"#)?;
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

    writeln!(file, r#"[command.deploy]"#)?;

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

    writeln!(file, r#"[command.test]"#)?;

    let result = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

// --- fallback_command promotion ---

// Same struct as above but tests use `fallback_command` in their config content.
// The config struct always has `fallback_command: Option<String>`. When set, the
// named command is promoted; when absent, config entries without a CLI command error.

#[derive(Debug, Parser, ConfigParser)]
struct PromotingCli {
    #[arg(short, long)]
    #[config(path, format = "toml")]
    config_path: String,
    #[command(subcommand)]
    command: PromotingCommand,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
enum PromotingCommand {
    Run(()),
    Build(BuildCommand),
    Debug {
        #[arg(long)]
        verbose: bool,
        #[arg(short, long, value_parser = humantime::parse_duration)]
        #[config(value_format = humantime::format_duration)]
        timeout: Option<Duration>,
    },
    Empty {},
    #[config(skip)]
    Test,
}

#[test]
fn fallback_command_promotes_unit_variant() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"command.fallback_command = "run""#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, PromotingCommand::Run(()));

    Ok(())
}

#[test]
fn fallback_command_promotes_newtype_variant() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"fallback_command = "build""#)?;
    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, PromotingCommand::Build(ref cmd) if cmd.target == "x86_64");

    Ok(())
}

#[test]
fn fallback_command_promotes_struct_variant() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"fallback_command = "debug""#)?;
    writeln!(file, r#"[command.debug]"#)?;
    writeln!(file, r#"verbose = true"#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(
        cli.command,
        PromotingCommand::Debug {
            verbose: true,
            timeout: None
        }
    );

    Ok(())
}

#[test]
fn fallback_command_promotes_without_entries() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"command.fallback_command = "run""#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, PromotingCommand::Run(()));

    Ok(())
}

#[test]
fn fallback_command_with_cli_command_still_wins() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"fallback_command = "run""#)?;
    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
        "build",
    ])?;

    assert_matches!(cli.command, PromotingCommand::Build(ref cmd) if cmd.target == "x86_64");

    Ok(())
}

#[test]
fn fallback_command_multi_entries_succeeds() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"fallback_command = "build""#)?;
    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;
    writeln!(file, r#"[command.debug]"#)?;
    writeln!(file, r#"verbose = true"#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_matches!(cli.command, PromotingCommand::Build(ref cmd) if cmd.target == "x86_64");

    Ok(())
}

#[test]
fn fallback_command_entries_without_fallback_errors() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;

    let result = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

#[test]
fn fallback_command_unknown_command_errors() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"command.fallback_command = "deploy""#)?;

    let result = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ]);

    assert!(result.is_err());

    Ok(())
}

#[test]
fn fallback_command_cli_override_with_fallback() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command]"#)?;
    writeln!(file, r#"fallback_command = "run""#)?;
    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;

    let cli = PromotingCli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
        "build",
        "--target",
        "arm64",
    ])?;

    assert_matches!(cli.command, PromotingCommand::Build(ref cmd) if cmd.target == "arm64");

    Ok(())
}
