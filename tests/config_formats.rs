use std::{fs, path::PathBuf};

use clap::{Parser, error::ErrorKind};
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config_path: Option<PathBuf>,
    #[arg(long)]
    port: u16,
    #[arg(long)]
    token: Option<String>,
}

fn write_config(suffix: &str, content: &str) -> Result<NamedTempFile> {
    let file = tempfile::Builder::new().suffix(suffix).tempfile()?;

    fs::write(file.path(), content)?;

    Ok(file)
}

fn parse(file: &NamedTempFile) -> Result<Cli, clap::Error> {
    let path = file.path().display().to_string();

    Cli::try_parse_with_config_from(["app", "--config-path", &path])
}

#[test]
fn format_is_inferred_from_extension() -> Result<()> {
    let configs = [
        (".toml", "port = 1\n"),
        (".yaml", "port: 1\n"),
        (".yml", "port: 1\n"),
        (".json", r#"{ "port": 1 }"#),
    ];

    for (suffix, content) in configs {
        let file = write_config(suffix, content)?;

        assert_eq!(parse(&file)?.port, 1, "{suffix}");
    }

    Ok(())
}

#[test]
fn unknown_extension_is_rejected() -> Result<()> {
    let file = write_config(".conf", "port = 1\n")?;
    let err = parse(&file).unwrap_err();

    assert_eq!(err.kind(), ErrorKind::InvalidValue);
    assert!(err.to_string().contains("unsupported format"), "{err}");

    Ok(())
}

#[test]
fn missing_file_names_path() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("missing-config.toml");
    let err =
        Cli::try_parse_with_config_from(["app", "--config-path", &path.display().to_string()])
            .unwrap_err();

    assert_eq!(err.kind(), ErrorKind::Io);
    assert!(err.to_string().contains("missing-config.toml"), "{err}");

    Ok(())
}

#[test]
fn invalid_file_names_path() -> Result<()> {
    let file = write_config(".toml", "port = \"x\"\n")?;
    let err = parse(&file).unwrap_err();
    let name = file.path().file_name().unwrap().to_string_lossy();

    assert_eq!(err.kind(), ErrorKind::InvalidValue);
    assert!(err.to_string().contains(name.as_ref()), "{err}");

    Ok(())
}

#[test]
fn yaml_type_error_is_readable() -> Result<()> {
    let file = write_config(".yaml", "port: 1\ntoken: [x]\n")?;
    let err = parse(&file).unwrap_err().to_string();

    assert!(err.contains("line 2"), "{err}");
    assert!(!err.contains("$__noyalib"), "{err}");

    Ok(())
}

#[test]
fn toml_error_hides_source_line() -> Result<()> {
    let file = write_config(".toml", "port = 1\ntoken = 123456789\n")?;
    let err = parse(&file).unwrap_err().to_string();

    assert!(err.contains("line 2"), "{err}");
    assert!(!err.contains("token = "), "{err}");

    Ok(())
}
