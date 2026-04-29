use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(short, long)]
    debug: bool,
    #[arg(short, long)]
    threads: u16,
    #[arg(short, long, default_value = "examples/config.toml")]
    #[config(path)]
    config_path: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
#[config(tag = "ref")]
enum Command {
    Test,
    Run(RunCommand),
    Debug {
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(Debug, Args, ConfigArgs)]
struct RunCommand {
    #[arg(long)]
    target: Option<String>,
}

fn main() {
    println!("{:#?}", Cli::parse_with_config());
}
