use std::{io::Write, path::PathBuf};

use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};
use eyre::Result;
use tempfile::NamedTempFile;

// Would clash with the default generated `CliConfig`.
#[allow(dead_code)]
struct CliConfig;

#[derive(Debug, Parser, ConfigParser)]
#[config(opts = "CliArgsOpts", config = "CliFile")]
struct Cli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[command(flatten)]
    server: Server,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args, ConfigArgs)]
#[config(opts = "ServerOptions", config = "ServerFile")]
struct Server {
    #[arg(long)]
    host: String,
    #[arg(long)]
    port: u16,
}

#[derive(Debug, PartialEq, Eq, Subcommand, ConfigSubcommand)]
#[config(opts = "CommandOptions", config = "CommandFile")]
enum Command {
    Run {
        #[arg(long)]
        jobs: u8,
    },
}

#[derive(Debug, Parser, ConfigParser)]
#[config(opts = "PartialOptions")]
struct Partial {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
}

#[test]
fn custom_names_avoid_clashes() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    write!(file, "host = \"h\"\n\n[command.run]\njobs = 4\n")?;

    let path = file.path().display().to_string();
    let cli =
        Cli::try_parse_with_config_from(["app", "--config-path", &path, "--port", "80", "run"])?;

    assert_eq!(cli.server.host, "h");
    assert_eq!(cli.server.port, 80);
    assert_eq!(cli.command, Command::Run { jobs: 4 });

    let _: Option<(CliFile, CliArgsOpts, ServerFile, ServerOptions)> = None;
    let _: Option<(CommandFile, CommandOptions)> = None;

    Ok(())
}

#[test]
fn one_key_keeps_other_default() {
    let _: Option<(PartialOptions, PartialConfig)> = None;
}
