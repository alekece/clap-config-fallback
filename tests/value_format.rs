use std::{io::Write, time::Duration};

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[arg(long, value_parser = humantime::parse_duration)]
    #[config(value_format = humantime::format_duration)]
    interval: Duration,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn formats_value_with_custom_formatter() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"interval = "1min""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.interval, Duration::from_secs(60));

    Ok(())
}
