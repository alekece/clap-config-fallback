use clap::Parser;
use clap_config_fallback::ConfigParser;

const NAME: &str = "app";

#[derive(Parser, ConfigParser)]
#[command(name = NAME)]
struct Cli {
    #[arg(long)]
    port: u16,
}

fn main() {}
