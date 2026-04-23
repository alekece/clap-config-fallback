use std::{ffi::OsString, iter, path::PathBuf};

use clap::{ArgMatches, CommandFactory, Error, Parser, error::ErrorKind};
use figment::{Figment, providers::*};
use serde::{Serialize, de::DeserializeOwned};

pub use clap_config_fallback_derive::ConfigParser;

/// Represents the supported configuration file formats.
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

/// Trait for converting parsed options into command-line arguments for the main parser.
pub trait IntoArgs {
    fn into_args(self) -> impl Iterator<Item = String>;
}

/// Trait for constructing an options struct from parsed command-line arguments, used by the
/// `ConfigParser` to create an instance of the options struct from the CLI arguments before
/// loading the configuration file.
pub trait FromArgs: Sized {
    fn from_args(args: &ArgMatches) -> Self;
}

/// Trait for types that can specify a configuration file path and format, used by the `ConfigParser`
/// to determine how to load configuration values from a file and merge them with CLI arguments.
pub trait ConfigSource {
    /// Returns the path to the configuration file, if specified.
    fn config_path(&self) -> Option<&str>;

    /// Returns the format of the configuration file, or `ConfigFormat::Unsupported`.
    fn config_format(&self) -> Option<ConfigFormat> {
        self.config_path().and_then(|path| {
            if cfg!(feature = "toml") && path.ends_with(".toml") {
                Some(ConfigFormat::Toml)
            } else if cfg!(feature = "yaml") && (path.ends_with(".yaml") || path.ends_with(".yml"))
            {
                Some(ConfigFormat::Yaml)
            } else if cfg!(feature = "json") && path.ends_with(".json") {
                Some(ConfigFormat::Json)
            } else {
                None
            }
        })
    }
}

pub trait ConfigParser: Sized + Parser {
    type Opts: Parser + Serialize + DeserializeOwned + IntoArgs + FromArgs + ConfigSource;
    type Config: Serialize + DeserializeOwned;

    fn parse_with_config() -> Self {
        Self::parse_with_config_from(std::env::args_os())
    }

    fn try_parse_with_config() -> Result<Self, Error> {
        Self::try_parse_with_config_from(std::env::args_os())
    }

    fn parse_with_config_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Self::try_parse_with_config_from(itr).unwrap_or_else(|e| e.exit())
    }

    fn try_parse_with_config_from<I, T>(itr: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let command = <Self::Opts as CommandFactory>::command();
        let args = command.try_get_matches_from(itr)?;
        let opts = Self::Opts::from_args(&args);

        let config = opts.config_path().map(|path| {
            let path = PathBuf::from(path);

            if !path.exists() {
                return Err(Self::command().error(ErrorKind::Io, "configuration file not found"));
            }

            let config = match opts.config_format() {
                #[cfg(feature = "toml")]
                Some(ConfigFormat::Toml) => Figment::from(Toml::file(path)),
                #[cfg(feature = "yaml")]
                Some(ConfigFormat::Yaml) => Figment::from(Yaml::file(path)),
                #[cfg(feature = "json")]
                Some(ConfigFormat::Json) => Figment::from(Json::file(path)),
                _ => {
                    return Err(Self::command().error(
                        ErrorKind::InvalidValue,
                        "unsupported configuration file".to_string(),
                    ));
                }
            };

            config.extract::<Self::Config>().map_err(|e| {
                Self::command().error(
                    ErrorKind::InvalidValue,
                    format!("invalid configuration file: {e}"),
                )
            })
        });

        let mut cli = Figment::from(Serialized::defaults(opts));

        if let Some(config) = config {
            cli = cli.join(Serialized::defaults(config?));
        }

        let cli = cli
            .extract::<Self::Opts>()
            .map_err(|e| Self::command().error(ErrorKind::InvalidValue, e.to_string()))?;

        Self::try_parse_from(iter::once(env!("CARGO_PKG_NAME").to_string()).chain(cli.into_args()))
    }
}
