//! `clap-config-fallback` extends clap with configuration-file fallback while preserving clap as
//! the final parser/validator.
//!
//! Typical flow:
//! 1. Parse CLI args into an optional intermediate struct.
//! 2. Load config values from `#[config(path)]` (if provided).
//! 3. Merge with precedence `CLI > config`.
//! 4. Re-run clap on reconstructed arguments for final validation.

use std::{ffi::OsString, iter, path::PathBuf};

use clap::{ArgMatches, Args, CommandFactory, Error, Parser, Subcommand, error::ErrorKind};
use figment::{Figment, providers::*};
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "derive")]
pub use clap_config_fallback_derive::{ConfigArgs, ConfigParser, ConfigSubcommand};

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

/// Converts an intermediate options struct into synthetic CLI args.
///
/// These args are fed back into clap for the final parse/validation pass.
pub trait IntoArgs {
    fn into_args(self) -> impl Iterator<Item = String>;
}

/// Builds an intermediate options struct from clap `ArgMatches`.
///
/// This pass captures values explicitly provided on the CLI before config fallback is applied.
pub trait FromArgs: Sized {
    fn from_args(args: &ArgMatches) -> Option<Self>;
}

/// Provides configuration path and format discovery for fallback loading.
pub trait ConfigSource {
    /// Returns the config file path when fallback should be attempted.
    fn config_path(&self) -> Option<&str>;

    /// Returns a config format if it can be resolved.
    ///
    /// The default implementation infers from extension:
    /// - `.toml` (`toml` feature)
    /// - `.yaml` / `.yml` (`yaml` feature)
    /// - `.json` (`json` feature)
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

/// Companion trait for clap [`Args`] types participating in config fallback.
///
/// This trait is implemented by `#[derive(ConfigArgs)]`.
pub trait ConfigArgs: Sized + Args {
    /// Intermediate optional representation used during merge.
    type Opts: Args + Serialize + DeserializeOwned + IntoArgs + FromArgs;
    /// Config-only representation loaded from file.
    type Config: Serialize + DeserializeOwned;
}

/// Companion trait for clap [`Subcommand`] types participating in config fallback.
///
/// This trait is implemented by `#[derive(ConfigSubcommand)]`.
pub trait ConfigSubcommand: Sized + Subcommand {
    /// Intermediate optional representation used during merge.
    type Opts: Subcommand + Serialize + DeserializeOwned + IntoArgs + FromArgs;
    /// Config-only representation loaded from file.
    type Config: Serialize + DeserializeOwned;
}

/// Parse a clap struct with optional configuration-file fallback.
///
/// Deriving `ConfigParser` generates an internal optional `Opts` type and a config-deserialization
/// type, then wires them into this trait.
pub trait ConfigParser: Sized + Parser {
    /// Intermediate optional representation used for CLI/config merge.
    type Opts: Parser + Serialize + DeserializeOwned + IntoArgs + FromArgs + Default + ConfigSource;
    /// Config-only representation loaded from file.
    type Config: Serialize + DeserializeOwned;

    /// Equivalent to [`Parser::parse`], but with config fallback.
    fn parse_with_config() -> Self {
        Self::parse_with_config_from(std::env::args_os())
    }

    /// Equivalent to [`Parser::try_parse`], but with config fallback.
    fn try_parse_with_config() -> Result<Self, Error> {
        Self::try_parse_with_config_from(std::env::args_os())
    }

    /// Equivalent to [`Parser::parse_from`], but with config fallback.
    fn parse_with_config_from<I, T>(itr: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Self::try_parse_with_config_from(itr).unwrap_or_else(|e| e.exit())
    }

    /// Performs parsing with config fallback and returns clap errors instead of exiting.
    ///
    /// Merge precedence is **CLI > config**. If no config path is available, behavior matches a
    /// normal clap parse.
    fn try_parse_with_config_from<I, T>(itr: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let command = <Self::Opts as CommandFactory>::command();
        let command_name = command.get_name().to_owned();
        let args = command.try_get_matches_from(itr)?;
        let opts = Self::Opts::from_args(&args).unwrap_or_default();

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

        Self::try_parse_from(iter::once(command_name).chain(cli.into_args()))
    }
}
