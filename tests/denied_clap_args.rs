use std::io::Write;

use clap::{Args, Parser, error::ErrorKind};
use clap_config_fallback::{ConfigArgs, ConfigParser};
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
    #[arg(long, requires_all = ["age", "size"])]
    username: Option<String>,
    #[command(flatten)]
    user: UserArgs,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[derive(Debug, Args, ConfigArgs, PartialEq, Eq)]
struct UserArgs {
    #[arg(long, default_value_t = 42u16)]
    age: u16,
    #[arg(long, env = "TEST_SIZE")]
    size: Option<u32>,
}

#[test]
fn denied_env_fallback_can_be_satisfied_by_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"size = 42"#)?;

    unsafe { std::env::set_var("TEST_SIZE", "14") };

    let cli = Cli::try_parse_with_config_from([
        "bin",
        "--value",
        "from-cli",
        "--config-path",
        &file.path().display().to_string(),
    ])?;

    assert_eq!(cli.value, "from-cli");
    assert_eq!(cli.user.size, Some(42));

    Ok(())
}

#[test]
fn denied_required_arg_can_be_satisfied_by_config() -> Result<()> {
    let mut file = NamedTempFile::new()?;

    writeln!(file, r#"value = "configured""#)?;
    writeln!(file, r#"token = "abc""#)?;
    writeln!(file, r#"username = "user""#)?;
    writeln!(file, r#"age = 30"#)?;

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
