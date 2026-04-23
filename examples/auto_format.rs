use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(short, long)]
    profile: String,
    #[arg(short, long)]
    threads: usize,
    // The "auto" format will try to infer the format from the file extension.
    // This is useful when you want to support multiple config formats without having to specify the
    // format explicitly.
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "auto")]
    config_path: String,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
