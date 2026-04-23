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
    #[command(flatten)]
    server: ServerCli,
    #[arg(long, default_value = "examples/config.toml")]
    #[config(path, format = "toml")]
    config_path: String,
}

#[derive(Debug, Parser, ConfigParser)]
struct ServerCli {
    #[arg(long)]
    url: String,
    #[command(flatten)]
    tls: TlsCli,
}

#[derive(Debug, Parser, ConfigParser)]
struct TlsCli {
    #[arg(long)]
    enabled: bool,
    #[arg(long)]
    cert_path: String,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
