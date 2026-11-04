use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Parser, ConfigParser)]
#[config(opts = "not a type")]
struct Cli {
    #[arg(long)]
    port: u16,
}

fn main() {}
