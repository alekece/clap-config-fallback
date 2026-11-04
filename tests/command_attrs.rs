use std::{fs, io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use clap_config_fallback::{ConfigParser, ConfigSubcommand};
use eyre::Result;
use tempfile::NamedTempFile;

fn write_config(content: &str) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;

    write!(file, "{content}")?;

    Ok(file)
}

fn path_of(file: &NamedTempFile) -> String {
    file.path().display().to_string()
}

#[derive(Debug, Parser, ConfigParser)]
#[command(rename_all = "snake_case")]
struct SnakeCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    max_conn: u16,
}

#[test]
fn struct_rename_all_round_trips() -> Result<()> {
    let cli = SnakeCli::try_parse_with_config_from(["app", "--max_conn", "3"])?;

    assert_eq!(cli.max_conn, 3);

    let file = write_config("max_conn = 4\n")?;
    let cli = SnakeCli::try_parse_with_config_from(["app", "--config_path", &path_of(&file)])?;

    assert_eq!(cli.max_conn, 4);

    Ok(())
}

#[derive(Debug, Parser, ConfigParser)]
#[command(arg_required_else_help = true)]
struct RequiredElseHelpCli {
    #[arg(long, default_value = REQUIRED_ELSE_HELP_CONFIG)]
    #[config(path, format = "toml")]
    config_path: PathBuf,
    #[arg(long)]
    port: u16,
}

const REQUIRED_ELSE_HELP_CONFIG: &str =
    concat!(env!("CARGO_TARGET_TMPDIR"), "/required-else-help.toml");

#[test]
fn arg_required_else_help_reads_default_config() -> Result<()> {
    fs::write(REQUIRED_ELSE_HELP_CONFIG, "port = 7\n")?;

    let cli = RequiredElseHelpCli::try_parse_with_config_from(["app"]);

    assert_eq!(cli?.port, 7);

    Ok(())
}

#[derive(Debug, Parser, ConfigParser)]
struct RenamedCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[command(subcommand)]
    cmd: RenamedCommand,
}

#[derive(Debug, PartialEq, Eq, Subcommand, ConfigSubcommand)]
enum RenamedCommand {
    #[command(name = "go")]
    Run {
        #[arg(long)]
        n: u8,
    },
}

#[test]
fn renamed_variant_uses_cli_name() -> Result<()> {
    let file = write_config("[cmd.go]\nn = 3\n")?;
    let cli =
        RenamedCli::try_parse_with_config_from(["app", "--config-path", &path_of(&file), "go"])?;

    assert_eq!(cli.cmd, RenamedCommand::Run { n: 3 });

    Ok(())
}

#[derive(Debug, Parser, ConfigParser)]
struct SnakeCommandCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[command(subcommand)]
    cmd: SnakeCommand,
}

#[derive(Debug, PartialEq, Eq, Subcommand, ConfigSubcommand)]
#[command(rename_all = "snake_case")]
enum SnakeCommand {
    DryRun {
        #[arg(long)]
        max_conn: u8,
    },
}

#[test]
fn enum_rename_all_applies_to_variant_fields() -> Result<()> {
    let cli = SnakeCommandCli::try_parse_with_config_from(["app", "dry_run", "--max_conn", "2"])?;

    assert_eq!(cli.cmd, SnakeCommand::DryRun { max_conn: 2 });

    let file = write_config("[cmd.dry_run]\nmax_conn = 5\n")?;
    let cli = SnakeCommandCli::try_parse_with_config_from([
        "app",
        "--config-path",
        &path_of(&file),
        "dry_run",
    ])?;

    assert_eq!(cli.cmd, SnakeCommand::DryRun { max_conn: 5 });

    Ok(())
}
