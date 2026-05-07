use clap::{Parser, error::ErrorKind};
use clap_config_fallback::ConfigParser;
use eyre::Result;
use std::io::Write;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    name: String,
    #[arg(long)]
    threads: u16,
    #[arg(long)]
    debug: bool,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn uses_config_values_when_cli_omits_them() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"name = "from-config""#)?;
    writeln!(file, "threads = 4")?;
    writeln!(file, "debug = true")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.name, "from-config");
    assert_eq!(cli.threads, 4);
    assert!(cli.debug);

    Ok(())
}

#[test]
fn cli_overrides_config_values() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"name = "from-config""#)?;
    writeln!(file, "threads = 2")?;
    writeln!(file, "debug = false")?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "from-cli",
        "--threads",
        "8",
        "--debug",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.name, "from-cli");
    assert_eq!(cli.threads, 8);
    assert!(cli.debug);

    Ok(())
}

#[test]
fn missing_config_file_is_reported_as_io_error() -> Result<()> {
    let config_path = "missing.toml";

    let err = Cli::try_parse_with_config_from([
        "bin",
        "x",
        "--threads",
        "1",
        "--config-path",
        config_path,
    ])
    .expect_err("missing file should fail");

    assert_eq!(err.kind(), ErrorKind::Io);

    Ok(())
}
