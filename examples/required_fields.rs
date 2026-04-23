use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(short, long, required = true)]
    #[config(skip)]
    profile: Option<String>,
    #[arg(short, long, required = true)]
    threads: Option<usize>,
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "toml")]
    config_path: String,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
