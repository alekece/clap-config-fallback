use std::io::Write;

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

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
        value_parser = ["trace", "debug", "info", "warn", "error"],
        ignore_case = true
    )]
    log_level: String,
    #[arg(long, value_name = "MS")]
    timeout_ms: Option<u64>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn allowed_clap_args_are_forwarded_to_opts() -> Result<()> {
    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--worker-threads",
        "7",
        "--log",
        "DEBUG",
        "--timeout-ms",
        "1500",
    ])?;

    assert_eq!(cli.threads, 7);
    assert_eq!(cli.log_level, "DEBUG");
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
