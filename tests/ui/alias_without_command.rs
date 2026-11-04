use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(alias = "p")]
    port: u16,
}

fn main() {}
