use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    database: DatabaseCli,
    #[arg(long, default_value = "examples/config.toml")]
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
#[config(tag = "ref")]
enum Command {
    Test {
        #[arg(long, value_parser = humantime::parse_duration)]
        #[config(value_format = humantime::format_duration)]
        duration: Duration,
    },
}

#[derive(Debug, Args, ConfigArgs)]
struct PoolCli {
    #[arg(long)]
    max_connections: u16,
    #[arg(long, value_parser = humantime::parse_duration)]
    #[config(value_format = humantime::format_duration)]
    timeout: Duration,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
