use clap::Parser;
use clap_config_fallback::ConfigParser;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Custom(String);

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    name: String,
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    profile: String,
    #[arg(long)]
    threads: usize,
    #[arg(short, long, value_parser = from_uppercase, value_delimiter = ',')]
    #[config(value_format = to_uppercase)]
    names: Vec<Custom>,
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "toml")]
    config_path: String,
}

fn from_uppercase(value: &str) -> Result<Custom, String> {
    Ok(Custom(value.to_lowercase()))
}

fn to_uppercase(value: Custom) -> String {
    value.0.to_uppercase()
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
