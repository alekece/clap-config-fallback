use std::time::Duration;

use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    database: DatabaseCli,
    #[arg(long, default_value = "examples/config.json")]
    #[config(path)]
    config_path: String,
}

#[derive(Debug, Parser, ConfigParser)]
struct DatabaseCli {
    #[arg(long)]
    url: String,
    #[command(flatten)]
    pool: PoolCli,
}

#[derive(Debug, Parser, ConfigParser)]
struct PoolCli {
    #[arg(long)]
    max_connections: u16,
    #[arg(long, value_parser = humantime::parse_duration)]
    #[config(value_format = humantime::format_duration(timeout).to_string())]
    timeout: Duration,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
