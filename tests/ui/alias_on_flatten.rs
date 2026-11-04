use clap::{Args, Parser};
use clap_config_fallback::{ConfigArgs, ConfigParser};

#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    #[config(alias = "db")]
    database: Database,
}

#[derive(Args, ConfigArgs)]
struct Database {
    #[arg(long)]
    host: String,
}

fn main() {}
