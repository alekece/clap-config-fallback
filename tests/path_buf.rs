use std::path::PathBuf;

use clap::Parser;
use clap_config_fallback::ConfigParser;
use eyre::Result;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    input_file: PathBuf,
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
    #[arg(short, long, value_delimiter = ',')]
    log_files: Option<Vec<PathBuf>>,
    #[arg(long)]
    #[config(path, format = "toml")]
    config_path: Option<String>,
}

#[test]
fn path_buf_is_parsed_from_cli() -> Result<()> {
    let cli = Cli::try_parse_with_config_from(["bin", "my_path", "-l", "log1,log2"])?;

    assert_eq!(cli.input_file, PathBuf::from("my_path"));
    assert_eq!(cli.output_dir, None);
    assert_eq!(
        cli.log_files.as_ref().unwrap().as_slice(),
        [PathBuf::from("log1"), PathBuf::from("log2")]
    );

    Ok(())
}
