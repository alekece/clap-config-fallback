use std::io::Write;

use clap::{Args, Error, Parser, error::ErrorKind};
use clap_config_fallback::{ConfigArgs, ConfigParser};
use eyre::Result;
use tempfile::NamedTempFile;

fn parse_log(log: &str) -> Result<String, Error> {
    if ["trace", "debug", "info", "warn", "error"].contains(&log.to_lowercase().as_str()) {
        Ok(log.to_string())
    } else {
        Err(Error::new(ErrorKind::InvalidValue))
    }
}

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[arg(short = 't', long = "threads", alias = "worker-threads")]
    threads: u16,
    #[arg(
        short = 'l',
        long = "log-level",
        alias = "verbosity",
        aliases = ["log", "verbosity"],
        default_value = "info",
        value_parser = parse_log,
        ignore_case = true
    )]
    log_level: String,
    #[arg(long, value_name = "MS")]
    timeout_ms: Option<u64>,
    #[command(flatten)]
    args: CliArgs,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[derive(Debug, Args, ConfigArgs, PartialEq, Eq)]
struct CliArgs {
    #[arg(long)]
    url: Option<String>,
}

#[test]
fn forwards_flattened_fields_to_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "threads = 5")?;
    writeln!(file, "log_level = 'warn'")?;
    writeln!(file, "timeout_ms = 2000")?;
    writeln!(file, "url = 'https://example.com'")?;

    let cli =
        Cli::try_parse_with_config_from(["bin", "--config-path", file.path().to_str().unwrap()])?;

    assert_eq!(cli.threads, 5);
    assert_eq!(cli.log_level, "warn");
    assert_eq!(cli.timeout_ms, Some(2000));

    Ok(())
}

#[test]
fn forwards_value_parser_expr_path_to_config() -> Result<()> {
    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--worker-threads",
        "7",
        "--log",
        "DeBuG",
        "--timeout-ms",
        "1500",
    ])?;

    assert_eq!(cli.threads, 7);
    assert_eq!(cli.log_level, "DeBuG");
    assert_eq!(cli.timeout_ms, Some(1500));

    Ok(())
}

#[test]
fn forwards_aliases_to_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "verbosity = 'INFO'")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--threads",
        "4",
        "--config-path",
        file.path().to_str().unwrap(),
    ])?;

    assert_eq!(cli.log_level, "INFO");

    Ok(())
}
