use std::{io::Write, path::PathBuf};

use clap::{Parser, error::ErrorKind};
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

/// My tool about
#[derive(Debug, Parser, ConfigParser)]
#[command(version = "1.2.3")]
struct Cli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(long)]
    port: u16,
}

#[derive(Debug, Parser, ConfigParser)]
#[command(disable_help_flag = true)]
struct NoHelpCli {
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<PathBuf>,
    #[arg(short = 'h', long)]
    host: String,
    #[arg(long)]
    port: u16,
}

#[test]
fn short_h_user_flag_keeps_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "port = 9")?;

    let path = file.path().display().to_string();
    let cli = NoHelpCli::try_parse_with_config_from(["app", "-h", "x", "--config-path", &path])?;

    assert_eq!(cli.host, "x");
    assert_eq!(cli.port, 9);

    Ok(())
}

#[test]
fn help_uses_user_command() {
    let err = Cli::try_parse_with_config_from(["app", "--help"]).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
    assert!(err.to_string().contains("My tool about"));
}

#[test]
fn version_flag() {
    let err = Cli::try_parse_with_config_from(["app", "-V"]).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::DisplayVersion);
    assert!(err.to_string().contains("1.2.3"));
}

#[test]
fn help_after_a_value_is_still_help() {
    let err = Cli::try_parse_with_config_from(["app", "--port", "1", "-h"]).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::DisplayHelp);
}
