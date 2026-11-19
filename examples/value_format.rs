use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};

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

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    database: DatabaseCli,
    #[arg(long, default_value = "examples/config.json")]
    #[config(path)]
    config_path: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args, ConfigArgs)]
struct DatabaseCli {
    #[arg(long)]
    url: String,
    #[command(flatten)]
    pool: PoolCli,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
enum Command {
    Test {
        #[arg(long, value_parser = DurationFormat::parse)]
        #[config(value_format = DurationFormat::format)]
        duration: Duration,
    },
}

#[derive(Debug, Args, ConfigArgs)]
struct PoolCli {
    #[arg(long)]
    max_connections: u16,
    #[arg(long, value_parser = DurationFormat::parse)]
    #[config(value_format = DurationFormat::format)]
    timeout: Duration,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
