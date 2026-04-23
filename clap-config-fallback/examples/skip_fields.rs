use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(long)]
    #[config(skip)]
    profile: String,
    #[arg(long)]
    threads: usize,
    #[command(flatten)]
    database: DatabaseCli,
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "toml")]
    config_path: String,
}

#[derive(Debug, Parser, ConfigParser)]
struct DatabaseCli {
    #[arg(long)]
    url: String,
    #[command(flatten)]
    pool: PoolCli,
}

#[derive(Debug, Parser, ConfigParser)]
#[config(skip_all)]
struct PoolCli {
    #[arg(long)]
    max_connections: u16,
    #[arg(long)]
    timeout_seconds: u16,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
