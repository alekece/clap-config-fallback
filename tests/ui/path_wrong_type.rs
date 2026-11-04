use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config: u16,
}

fn main() {}
