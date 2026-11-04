use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Parser, ConfigParser)]
#[command(rename_all = "shouting")]
struct Cli {
    #[arg(long)]
    port: u16,
}

fn main() {}
