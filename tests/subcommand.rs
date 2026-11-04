use std::{io::Write, time::Duration};

use assert_matches::assert_matches;
use clap::{Args, Parser, Subcommand, error::ErrorKind};
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
    Deploy(DeployCommand),
    DryRun {
        #[arg(long)]
        limit: Option<u16>,
    },
    #[config(skip)]
    Test,
}

#[derive(Debug, Args, ConfigArgs)]
struct BuildCommand {
    #[arg(long)]
    target: String,
}

#[derive(Debug, Args, ConfigArgs)]
struct DeployCommand {
    #[arg(long)]
    region: String,
    #[command(subcommand)]
    command: DeployTarget,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
enum DeployTarget {
    Server {
        #[arg(long)]
        port: u16,
    },
    Client(ClientCommand),
}

#[derive(Debug, Args, ConfigArgs)]
struct ClientCommand {
    #[arg(long)]
    name: String,
}

fn write_config(lines: &[&str]) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;

    for line in lines {
        writeln!(file, "{line}")?;
    }

    Ok(file)
}

fn parse(file: &NamedTempFile, args: &[&str]) -> Result<Cli, clap::Error> {
    let path = file.path().display().to_string();
    let args = ["bin", "--config-path", path.as_str()]
        .into_iter()
        .chain(args.iter().copied());

    Cli::try_parse_with_config_from(args)
}

const MULTI_SECTION_CONFIG: &[&str] = &[
    "[command.build]",
    r#"target = "x86_64-unknown-linux-gnu""#,
    "[command.debug]",
    "verbose = true",
    r#"timeout = "5s""#,
    "[command.deploy]",
    r#"region = "eu-west""#,
    "[command.deploy.command.server]",
    "port = 8080",
    "[command.deploy.command.client]",
    r#"name = "desktop""#,
];

#[test]
fn newtype_variant_is_loaded_from_config() -> Result<()> {
    let file = write_config(&["[cmd.build]", r#"target = "x86_64-unknown-linux-gnu""#])?;

    let cli = parse(&file, &["build"])?;

    assert_matches!(cli.command, Command::Build(command) if command.target == "x86_64-unknown-linux-gnu");

    Ok(())
}

#[test]
fn struct_variant_is_loaded_from_config() -> Result<()> {
    let file = write_config(&["[action.debug]", "verbose = true"])?;

    let cli = parse(&file, &["debug"])?;

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
    let file = write_config(&["[command.build]", r#"target = "x86_64-unknown-linux-gnu""#])?;

    let cli = parse(&file, &["run"])?;

    assert_matches!(cli.command, Command::Run(()));

    Ok(())
}

#[test]
fn cli_command_wins_with_multiple_sections() -> Result<()> {
    let file = write_config(MULTI_SECTION_CONFIG)?;

    assert_matches!(
        parse(&file, &["build"])?.command,
        Command::Build(command) if command.target == "x86_64-unknown-linux-gnu"
    );
    assert_matches!(
        parse(&file, &["debug"])?.command,
        Command::Debug { verbose: true, timeout: Some(timeout) } if timeout == Duration::from_secs(5)
    );
    assert_matches!(parse(&file, &["run"])?.command, Command::Run(()));
    assert_matches!(parse(&file, &["empty"])?.command, Command::Empty {});

    Ok(())
}

#[test]
fn cli_values_override_selected_section() -> Result<()> {
    let file = write_config(MULTI_SECTION_CONFIG)?;

    let cli = parse(&file, &["build", "--target", "aarch64-apple-darwin"])?;

    assert_matches!(cli.command, Command::Build(command) if command.target == "aarch64-apple-darwin");

    Ok(())
}

#[test]
fn nested_subcommand_uses_selected_branch() -> Result<()> {
    let file = write_config(MULTI_SECTION_CONFIG)?;

    assert_matches!(
        parse(&file, &["deploy", "server"])?.command,
        Command::Deploy(DeployCommand { region, command: DeployTarget::Server { port: 8080 } })
            if region == "eu-west"
    );
    assert_matches!(
        parse(&file, &["deploy", "client"])?.command,
        Command::Deploy(DeployCommand { region, command: DeployTarget::Client(client) })
            if region == "eu-west" && client.name == "desktop"
    );
    assert_matches!(
        parse(&file, &["deploy", "--region", "us-east", "server", "--port", "9090"])?.command,
        Command::Deploy(DeployCommand { region, command: DeployTarget::Server { port: 9090 } })
            if region == "us-east"
    );

    Ok(())
}

#[test]
fn multi_word_command_section_is_kebab_case() -> Result<()> {
    let file = write_config(&["[command.dry-run]", "limit = 3"])?;

    assert_matches!(
        parse(&file, &["dry-run"])?.command,
        Command::DryRun { limit: Some(3) }
    );
    assert_matches!(
        parse(&file, &["dry-run", "--limit", "5"])?.command,
        Command::DryRun { limit: Some(5) }
    );

    Ok(())
}

#[test]
fn snake_case_command_section_is_accepted() -> Result<()> {
    let file = write_config(&["[command.dry_run]", "limit = 3"])?;

    let cli = parse(&file, &["dry-run"])?;

    assert_matches!(cli.command, Command::DryRun { limit: Some(3) });

    Ok(())
}

#[test]
fn both_command_section_spellings_returns_error() -> Result<()> {
    let file = write_config(&[
        "[command.dry_run]",
        "limit = 3",
        "[command.dry-run]",
        "limit = 4",
    ])?;

    let error = parse(&file, &["dry-run"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidValue);

    Ok(())
}

#[test]
fn no_cli_command_with_sections_returns_error() -> Result<()> {
    let file = write_config(MULTI_SECTION_CONFIG)?;

    let error = parse(&file, &[]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::MissingSubcommand);

    Ok(())
}

#[test]
fn no_cli_nested_command_with_sections_returns_error() -> Result<()> {
    let file = write_config(MULTI_SECTION_CONFIG)?;

    let error = parse(&file, &["deploy"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::MissingSubcommand);

    Ok(())
}

#[test]
fn config_cannot_select_command() -> Result<()> {
    let file = write_config(&[r#"command = "run""#])?;

    assert!(parse(&file, &[]).is_err());

    Ok(())
}

#[test]
fn unknown_command_section_returns_error() -> Result<()> {
    let file = write_config(&["[command.publish]"])?;

    let error = parse(&file, &["run"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidValue);

    Ok(())
}

#[test]
fn skipped_command_section_returns_error() -> Result<()> {
    let file = write_config(&["[command.test]"])?;

    let error = parse(&file, &["run"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidValue);

    Ok(())
}

#[test]
fn ill_typed_unselected_section_returns_error() -> Result<()> {
    let file = write_config(&["[command.debug]", r#"verbose = "yes""#])?;

    let error = parse(&file, &["build", "--target", "x86_64-unknown-linux-gnu"]).unwrap_err();

    assert_eq!(error.kind(), ErrorKind::InvalidValue);

    Ok(())
}
