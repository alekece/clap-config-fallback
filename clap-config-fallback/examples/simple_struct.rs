use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    threads: usize,
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "toml")]
    config_path: String,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
