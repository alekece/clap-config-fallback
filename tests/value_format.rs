use std::{io::Write, time::Duration};

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

struct DurationFormat;

impl DurationFormat {
    fn parse(value: &str) -> Result<Duration, String> {
        if let Some(seconds) = value.strip_suffix('s') {
            seconds
                .parse()
                .map(Duration::from_secs)
                .map_err(|error| error.to_string())
        } else if let Some(minutes) = value.strip_suffix("min") {
            minutes
                .parse::<u64>()
                .map(|minutes| Duration::from_secs(minutes * 60))
                .map_err(|error| error.to_string())
        } else {
            Err("duration must end in s or min".to_owned())
        }
    }

    fn format(duration: Duration) -> String {
        format!("{}s", duration.as_secs())
    }
}

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[arg(long, value_parser = DurationFormat::parse)]
    #[config(value_format = DurationFormat::format)]
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
