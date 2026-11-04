use clap_config_fallback::ConfigSubcommand;

#[derive(ConfigSubcommand)]
enum Command {
    Copy(String, String),
}

fn main() {}
