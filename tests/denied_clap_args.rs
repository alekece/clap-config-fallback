use std::io::Write;

use clap::{error::ErrorKind, Parser};
use clap_config_fallback::ConfigParser;
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser, PartialEq, Eq)]
struct Cli {
    #[arg(long, required = true)]
    value: String,
    #[arg(long, default_value = "default-from-clap")]
    mode: String,
    #[arg(long, conflicts_with = "right")]
    left: bool,
    #[arg(long)]
    right: bool,
    #[arg(long, requires = "username")]
    token: Option<String>,
    #[arg(long)]
    username: Option<String>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn denied_required_arg_can_be_satisfied_by_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"value = "configured""#)?;
    writeln!(file, r#"token = "abc""#)?;
    writeln!(file, r#"username = "user""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.value, "configured");
    assert_eq!(cli.token.as_deref(), Some("abc"));
    assert_eq!(cli.username.as_deref(), Some("user"));

    Ok(())
}

#[test]
fn denied_required_arg_still_errors_after_merge_when_missing() -> Result<()> {
    let err = Cli::try_parse_with_config_from(["bin"])
        .expect_err("missing required arguments should fail");

    assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);

    Ok(())
}

#[test]
fn denied_default_value_does_not_override_config_value() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"mode = "configured-mode""#)?;

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--value",
        "explicit",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.value, "explicit");
    assert_eq!(cli.mode, "configured-mode");

    Ok(())
}

#[test]
fn denied_conflicts_with_is_enforced_on_final_parse() -> Result<()> {
    let err = Cli::try_parse_with_config_from(["bin", "--left", "--right"])
        .expect_err("conflicting arguments should fail");

    assert_eq!(err.kind(), clap::error::ErrorKind::ArgumentConflict);

    Ok(())
}

#[test]
fn denied_requires_is_enforced_on_final_parse() -> Result<()> {
    let err = Cli::try_parse_with_config_from(["bin", "--token", "abc123"])
        .expect_err("missing requires arguments should fail");

    assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);

    Ok(())
}
