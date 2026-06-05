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
    command: Command,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
enum Command {
    Build(BuildCommand),
    Run(RunCommand),
    Debug(DebugCommand),
}

#[derive(Debug, Args, ConfigArgs)]
struct BuildCommand {
    #[arg(long)]
    target: String,
}

#[derive(Debug, Args, ConfigArgs)]
struct RunCommand {
    #[arg(long)]
    extra_flag: bool,
}

#[derive(Debug, Args, ConfigArgs)]
struct DebugCommand {
    #[arg(long)]
    verbose: bool,
    #[arg(short, long, value_parser = humantime::parse_duration)]
    #[config(value_format = humantime::format_duration)]
    timeout: Option<Duration>,
}

fn write_config() -> Result<(NamedTempFile, String)> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"[command.build]"#)?;
    writeln!(file, r#"target = "x86_64""#)?;
    writeln!(file, r#"[command.run]"#)?;
    writeln!(file, r#"extra_flag = true"#)?;
    writeln!(file, r#"[command.debug]"#)?;
    writeln!(file, r#"verbose = true"#)?;
    writeln!(file, r#"timeout = "30s""#)?;

    let path = file.path().display().to_string();
    Ok((file, path))
}

/// When the CLI does not select any subcommand and the config has entries
/// for multiple subcommands, this should error — the config is ambiguous
/// about which command to run.
#[test]
fn no_cli_command_with_multi_command_config_should_error() -> Result<()> {
    let (_file, path) = write_config()?;

    let result = Cli::try_parse_with_config_from(["bin", "--config-path", &path]);

    assert!(result.is_err());

    Ok(())
}

/// CLI selects `run` with no extra args. Config provides `extra_flag = true`
/// for the `run` command. The CLI command selection should be preserved,
/// and the config should apply its fallback to the matching command.
#[test]
fn cli_run_gets_config_fallback_for_run() -> Result<()> {
    let (_file, path) = write_config()?;

    let cli = Cli::try_parse_with_config_from(["bin", "--config-path", &path, "run"])?;

    assert_matches!(cli.command, Command::Run(ref cmd) if cmd.extra_flag);

    Ok(())
}

/// CLI selects `run --extra-flag`. The explicit CLI value should take
/// precedence over the config fallback of `extra_flag = true`.
#[test]
fn cli_run_explicit_flag_overrides_config_fallback() -> Result<()> {
    let (_file, path) = write_config()?;

    let cli =
        Cli::try_parse_with_config_from(["bin", "--config-path", &path, "run", "--extra-flag"])?;

    assert_matches!(cli.command, Command::Run(ref cmd) if cmd.extra_flag);

    Ok(())
}

/// CLI selects `debug --verbose`. Config provides `verbose = true` for
/// `debug`. Using `debug` (alphabetically after `build`) avoids false
/// negatives where the config happens to pick the same variant.
#[test]
fn cli_debug_explicit_flag_with_multi_command_config() -> Result<()> {
    let (_file, path) = write_config()?;

    let cli =
        Cli::try_parse_with_config_from(["bin", "--config-path", &path, "debug", "--verbose"])?;

    assert_matches!(cli.command, Command::Debug(ref cmd) if cmd.verbose && cmd.timeout == Some(std::time::Duration::from_secs(30)));

    Ok(())
}
