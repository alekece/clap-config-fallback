//! `clap-config-fallback` extends clap with configuration-file fallback while preserving clap as
//! the final parser/validator.
//!
//! Typical flow:
//! 1. Parse CLI args into an optional intermediate struct.
//! 2. Load config values from `#[config(path)]` (if provided).
//! 3. Merge with precedence `CLI > config`.
//! 4. Re-run clap on reconstructed arguments for final validation.

mod arg;
pub mod format;

use std::{
    ffi::{OsStr, OsString},
    fs, iter,
    path::Path,
};

use clap::{CommandFactory, Error, Parser, error::ErrorKind};
use figment::{Figment, providers::*};
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "derive")]
pub use clap_config_fallback_derive::{ConfigArgs, ConfigParser, ConfigSubcommand};

pub use arg::{Arg, FromArgs, IntoArgs};

/// Dependencies used by derive-generated code, so users do not have to depend on them directly.
///
/// Not part of the public API.
#[doc(hidden)]
pub mod reexports {
    pub use serde;
}

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

/// Provides configuration path and format discovery for fallback loading.
pub trait ConfigSource {
    /// Returns the config file path when fallback should be attempted.
    fn config_path(&self) -> Option<&Path>;

    /// Returns a config format if it can be resolved.
    ///
    /// The default implementation infers from extension:
    /// - `.toml` (`toml` feature)
    /// - `.yaml` / `.yml` (`yaml` feature)
    /// - `.json` (`json` feature)
    fn config_format(&self) -> Option<ConfigFormat> {
        self.config_path()
            .and_then(|path| match path.extension().and_then(OsStr::to_str) {
                #[cfg(feature = "toml")]
                Some("toml") => Some(ConfigFormat::Toml),
                #[cfg(feature = "yaml")]
                Some("yaml" | "yml") => Some(ConfigFormat::Yaml),
                #[cfg(feature = "json")]
                Some("json") => Some(ConfigFormat::Json),
                _ => None,
            })
    }
}

/// Defines the associated types for config fallback parsing.
pub trait ConfigFallback {
    /// Intermediate optional representation used during merge.
    type Opts: Serialize + DeserializeOwned + IntoArgs + FromArgs;
    /// Config-only representation loaded from file.
    ///
    /// It must be `'static` so YAML errors can report line and column.
    type Config: Serialize + DeserializeOwned + 'static;

    /// Drops config sections of subcommands that were not selected on the CLI.
    ///
    /// The CLI alone selects subcommands: config only supplies parameters for the selected branch.
    #[doc(hidden)]
    fn retain_commands(_config: &mut Self::Config, _opts: &Self::Opts) {}
}

/// Parse a clap struct with optional configuration-file fallback.
///
/// Deriving `ConfigParser` generates an internal optional `Opts` type and a config-deserialization
/// type, then wires them into this trait.
pub trait ConfigParser: Sized + Parser + ConfigFallback
where
    Self::Opts: Parser + Default + ConfigSource,
{
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
    fn try_parse_with_config_from<I, T>(args: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        let command = <Self::Opts as CommandFactory>::command();
        let command_name = command.get_name().to_owned().into();
        let args = args.into_iter().collect::<Vec<_>>();

        // Help and version must come from the user's command: it has the doc comments and the
        // required args that the all-optional `Opts` command lacks.
        let args = match command.try_get_matches_from(args.clone()) {
            Ok(args) => args,
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::DisplayHelp
                        | ErrorKind::DisplayVersion
                        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
                ) =>
            {
                return Self::try_parse_from(args);
            }
            Err(e) => return Err(e),
        };

        let opts = Self::Opts::from_args(&args).unwrap_or_default();

        let mut config: Option<Self::Config> = opts
            .config_path()
            .map(|path| {
                let content = fs::read_to_string(path).map_err(|e| {
                    Self::command().error(
                        ErrorKind::Io,
                        format!("failed to read configuration file {}: {e}", path.display()),
                    )
                })?;

                match opts.config_format() {
                    #[cfg(feature = "toml")]
                    Some(ConfigFormat::Toml) => toml::from_str(&content).map_err(|e| {
                        // toml's `Display` quotes the offending line, which may hold a secret
                        let Some(span) = e.span() else {
                            return e.message().to_owned();
                        };

                        let before = &content[..span.start];
                        let line = before.matches('\n').count() + 1;
                        let column = before
                            .rsplit('\n')
                            .next()
                            .unwrap_or_default()
                            .chars()
                            .count()
                            + 1;

                        format!("line {line}, column {column}: {}", e.message())
                    }),
                    #[cfg(feature = "yaml")]
                    Some(ConfigFormat::Yaml) => {
                        noyalib::from_str(&content).map_err(|e| e.to_string())
                    }
                    #[cfg(feature = "json")]
                    Some(ConfigFormat::Json) => {
                        serde_json::from_str(&content).map_err(|e| e.to_string())
                    }
                    _ => Err("unsupported format".to_string()),
                }
                .map_err(|e| {
                    Self::command().error(
                        ErrorKind::InvalidValue,
                        format!("invalid configuration file {}: {e}", path.display()),
                    )
                })
            })
            .transpose()?;

        // The whole file is validated above; only the CLI-selected branch takes part in the merge.
        if let Some(config) = &mut config {
            Self::retain_commands(config, &opts);
        }

        let mut cli = Figment::from(Serialized::defaults(opts));

        if let Some(config) = config {
            cli = cli.join(Serialized::defaults(config));
        }

        let cli = cli
            .extract::<Self::Opts>()
            .map_err(|e| Self::command().error(ErrorKind::InvalidValue, e.to_string()))?;

        Self::try_parse_from(iter::once(command_name).chain(cli.into_args()))
    }
}
