use std::io::Write;

use clap::{Args, Parser, Subcommand};
use clap_config_fallback::{ConfigArgs, ConfigParser, ConfigSubcommand};
use eyre::Result;
use tempfile::NamedTempFile;

#[derive(Debug, Parser, ConfigParser)]
struct DashCli {
    #[arg(long)]
    offset: i32,
    #[arg(short)]
    name: String,
    #[arg(long)]
    deltas: Vec<i32>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: String,
}

#[derive(Debug, Parser, ConfigParser)]
struct IdCli {
    #[arg(long)]
    r#type: Option<String>,
    #[arg(short)]
    r#match: Option<String>,
    #[arg(long, id = "custom-id")]
    renamed: Option<String>,
    #[arg(id = "snake_id", long)]
    long_from_id: Option<String>,
    #[arg(id = "zed", short)]
    short_from_id: Option<String>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[derive(Debug, Parser, ConfigParser)]
struct PositionalCli {
    #[command(flatten)]
    inner: PositionalArgs,
    second: Option<String>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[derive(Debug, Args, ConfigArgs)]
struct PositionalArgs {
    first: Option<String>,
}

#[derive(Debug, Parser, ConfigParser)]
struct OrderCli {
    #[command(flatten)]
    app: AppArgs,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: String,
    #[arg(long)]
    threads: u16,
}

#[derive(Debug, Args, ConfigArgs)]
struct AppArgs {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand, ConfigSubcommand)]
enum Command {
    Run {
        #[arg(long)]
        target: Option<String>,
    },
}

fn write_config(lines: &[&str]) -> Result<NamedTempFile> {
    let mut file = NamedTempFile::new()?;

    for line in lines {
        writeln!(file, "{line}")?;
    }

    Ok(file)
}

#[test]
fn accepts_dash_prefixed_config_values() -> Result<()> {
    let file = write_config(&["offset = -5", r#"name = "-x""#, "deltas = [-1, 2, -3]"])?;
    let path = file.path().display().to_string();

    let cli = DashCli::try_parse_with_config_from(["bin", "--config-path", &path])?;

    assert_eq!(cli.offset, -5);
    assert_eq!(cli.name, "-x");
    assert_eq!(cli.deltas, [-1, 2, -3]);

    Ok(())
}

#[test]
fn reads_raw_and_custom_ids_from_cli() -> Result<()> {
    let cli = IdCli::try_parse_with_config_from([
        "bin",
        "--type",
        "a",
        "-m",
        "b",
        "--renamed",
        "c",
        "--snake_id",
        "d",
        "-z",
        "e",
    ])?;

    assert_eq!(cli.r#type.as_deref(), Some("a"));
    assert_eq!(cli.r#match.as_deref(), Some("b"));
    assert_eq!(cli.renamed.as_deref(), Some("c"));
    assert_eq!(cli.long_from_id.as_deref(), Some("d"));
    assert_eq!(cli.short_from_id.as_deref(), Some("e"));

    Ok(())
}

#[test]
fn reads_raw_and_custom_ids_from_config() -> Result<()> {
    let file = write_config(&[
        r#"type = "a""#,
        r#"match = "b""#,
        r#"renamed = "c""#,
        r#"long_from_id = "d""#,
        r#"short_from_id = "e""#,
    ])?;
    let path = file.path().display().to_string();

    let cli = IdCli::try_parse_with_config_from(["bin", "--config-path", &path])?;

    assert_eq!(cli.r#type.as_deref(), Some("a"));
    assert_eq!(cli.r#match.as_deref(), Some("b"));
    assert_eq!(cli.renamed.as_deref(), Some("c"));
    assert_eq!(cli.long_from_id.as_deref(), Some("d"));
    assert_eq!(cli.short_from_id.as_deref(), Some("e"));

    Ok(())
}

#[test]
fn accepts_fields_declared_after_flattened_subcommand() -> Result<()> {
    let file = write_config(&["threads = 4", "[command.run]", r#"target = "x""#])?;
    let path = file.path().display().to_string();

    let cli = OrderCli::try_parse_with_config_from(["bin", "--config-path", &path, "run"])?;

    assert_eq!(cli.threads, 4);
    assert!(matches!(cli.app.command, Command::Run { target: Some(ref t) } if t == "x"));

    Ok(())
}

#[test]
fn keeps_positional_order_across_flatten() -> Result<()> {
    let cli = PositionalCli::try_parse_with_config_from(["bin", "a", "b"])?;

    assert_eq!(cli.inner.first.as_deref(), Some("a"));
    assert_eq!(cli.second.as_deref(), Some("b"));

    let file = write_config(&[r#"first = "a""#, r#"second = "b""#])?;
    let path = file.path().display().to_string();

    let cli = PositionalCli::try_parse_with_config_from(["bin", "--config-path", &path])?;

    assert_eq!(cli.inner.first.as_deref(), Some("a"));
    assert_eq!(cli.second.as_deref(), Some("b"));

    Ok(())
}
