use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_config_fallback::{ConfigParser, ConfigSubcommand};

#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config_path: Option<PathBuf>,

    #[arg(long)]
    host: String,

    #[arg(long, value_parser = parse_port)]
    port: Option<u16>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, ConfigSubcommand)]
enum Command {
    Serve {
        #[arg(long)]
        workers: u16,
    },
}

fn parse_port(value: &str) -> Result<u16, String> {
    value.parse().map_err(|_| format!("invalid port: {value}"))
}

fn main() {
    let _ = Cli::parse_with_config();
}
