use std::io::Write;

use clap::{Args, Error, Parser, error::ErrorKind};
use clap_config_fallback::{ConfigArgs, ConfigParser};
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[command(flatten)]
    args: Option<arg::CliArgs>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

pub mod arg {
    use super::*;

    #[derive(Debug, Args, ConfigArgs, PartialEq, Eq)]
    pub struct CliArgs {
        #[arg(long)]
        pub debug: bool,
        #[arg(long, value_parser = non_empty_string)]
        pub level: String,
    }

    fn non_empty_string(s: &str) -> Result<String, Error> {
        (!s.is_empty())
            .then(|| s.to_string())
            .ok_or_else(|| Error::new(ErrorKind::InvalidValue))
    }
}

#[test]
fn nested_fields_can_be_loaded_from_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, "[args]")?;
    writeln!(file, r#"level = "warn""#)?;
    writeln!(file, "debug = true")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    let args = cli.args.unwrap();

    assert_eq!(args.level, "warn");
    assert!(args.debug);

    Ok(())
}
