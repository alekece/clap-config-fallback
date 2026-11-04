use std::{fs, io::Write, path::PathBuf};

use clap::{ArgGroup, Parser, Subcommand};
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

#[derive(Debug, Parser, ConfigParser)]
struct VariantRenameCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[command(subcommand)]
    cmd: VariantRenameCommand,
}

#[derive(Debug, PartialEq, Eq, Subcommand, ConfigSubcommand)]
enum VariantRenameCommand {
    #[command(rename_all = "snake_case")]
    DryRun {
        #[arg(long)]
        max_conn: u8,
    },
}

#[test]
fn variant_rename_all_renames_the_subcommand() -> Result<()> {
    let cli = VariantRenameCli::try_parse_with_config_from(["app", "dry_run", "--max_conn", "1"])?;

    assert_eq!(cli.cmd, VariantRenameCommand::DryRun { max_conn: 1 });

    let file = write_config("[cmd.dry_run]\nmax_conn = 2\n")?;
    let cli = VariantRenameCli::try_parse_with_config_from([
        "app",
        "--config-path",
        &path_of(&file),
        "dry_run",
    ])?;

    assert_eq!(cli.cmd, VariantRenameCommand::DryRun { max_conn: 2 });

    Ok(())
}

// clap accepts loose spellings such as "lowercase", and `lower` drops underscores from flags.
// Config keys still use the field name.
#[derive(Debug, Parser, ConfigParser)]
#[command(rename_all = "lowercase")]
struct LowerCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    max_conn: u8,
}

#[test]
fn lower_rename_all_matches_clap() -> Result<()> {
    let cli = LowerCli::try_parse_with_config_from(["app", "--maxconn", "3"])?;

    assert_eq!(cli.max_conn, 3);

    let file = write_config("max_conn = 4\n")?;
    let cli = LowerCli::try_parse_with_config_from(["app", "--configpath", &path_of(&file)])?;

    assert_eq!(cli.max_conn, 4);

    Ok(())
}

#[derive(Debug, Parser, ConfigParser)]
#[command(group = ArgGroup::new("target").required(true).args(["host", "socket"]))]
struct GroupCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    socket: Option<String>,
}

#[test]
fn required_group_can_be_satisfied_by_config() -> Result<()> {
    let file = write_config("host = \"h\"\n")?;
    let cli = GroupCli::try_parse_with_config_from(["app", "--config-path", &path_of(&file)])?;

    assert_eq!(cli.host.as_deref(), Some("h"));

    let err = GroupCli::try_parse_with_config_from(["app"]).unwrap_err();

    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    Ok(())
}

#[derive(Debug, Parser, ConfigParser)]
#[command(groups([ArgGroup::new("target").required(true).args(["host", "socket"])]))]
struct GroupsCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    socket: Option<String>,
}

#[test]
fn required_groups_can_be_satisfied_by_config() -> Result<()> {
    let file = write_config("socket = \"s\"\n")?;
    let cli = GroupsCli::try_parse_with_config_from(["app", "--config-path", &path_of(&file)])?;

    assert_eq!(cli.socket.as_deref(), Some("s"));

    let err = GroupsCli::try_parse_with_config_from(["app"]).unwrap_err();

    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    Ok(())
}

fn target_group() -> ArgGroup {
    ArgGroup::new("target")
        .required(true)
        .args(["host", "socket"])
}

#[derive(Debug, Parser, ConfigParser)]
#[command(group(&target_group()))]
struct GroupRefCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    host: Option<String>,
    #[arg(long)]
    socket: Option<String>,
}

#[test]
fn borrowed_required_group_can_be_satisfied_by_config() -> Result<()> {
    let file = write_config("host = \"h\"\n")?;
    let cli = GroupRefCli::try_parse_with_config_from(["app", "--config-path", &path_of(&file)])?;

    assert_eq!(cli.host.as_deref(), Some("h"));

    let err = GroupRefCli::try_parse_with_config_from(["app"]).unwrap_err();

    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    Ok(())
}
