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
    iter,
    path::{Path, PathBuf},
};

use clap::{CommandFactory, Error, Parser, error::ErrorKind};
use figment::{Figment, providers::*};
use serde::{Serialize, de::DeserializeOwned};

#[cfg(feature = "derive")]
pub use clap_config_fallback_derive::{ConfigArgs, ConfigParser, ConfigSubcommand};

pub use arg::{Arg, FromArgs, IntoArgs};

/// Supported configuration file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

/// Implemented by `*CommandConfig` struct types generated from `ConfigSubcommand`.
///
/// Merges config-provided fallback arguments into the CLI-selected command. Config acts purely as
/// fallback: the CLI-selected command always wins. Errors if the CLI provided no command but the
/// config has subcommand entries (ambiguous which command to run).
pub trait MergeCommand: Default + Sized {
    /// The opts-side command enum type.
    type CommandOpts;

    /// Merge config fallback into the CLI command selection.
    ///
    /// Returns an error string if the CLI provided no command but config has subcommand entries.
    fn merge_command(
        self,
        cli_command: Option<Self::CommandOpts>,
    ) -> Result<Option<Self::CommandOpts>, String>;
}

impl MergeCommand for () {
    type CommandOpts = ();

    fn merge_command(self, cli_command: Option<()>) -> Result<Option<()>, String> {
        Ok(cli_command)
    }
}

/// Connects the root CLI type to its subcommand field for command-aware merging.
///
/// Implemented by types deriving [`ConfigParser`] to expose extract/inject operations on the
/// subcommand field without going through Figment.
pub trait CommandAware: ConfigFallback {
    /// The opts-side enum type for the subcommand field, or `()` when none exists.
    type CommandOpts;
    /// The config-side struct type for the subcommand field, or `()` when none exists.
    type CommandConfig: MergeCommand<CommandOpts = Self::CommandOpts>;

    /// Removes and returns the command from opts, leaving `None` in its place.
    fn take_opts_command(opts: &mut Self::Opts) -> Option<Self::CommandOpts>;
    /// Removes and returns the command config from config, leaving `None` in its place.
    fn take_config_command(config: &mut Self::Config) -> Option<Self::CommandConfig>;
    /// Restores a resolved command back into opts.
    fn put_opts_command(opts: &mut Self::Opts, cmd: Option<Self::CommandOpts>);
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
    type Config: Serialize + DeserializeOwned;
}

/// Parse a clap struct with optional configuration-file fallback.
///
/// Deriving `ConfigParser` generates an internal optional `Opts` type and a config-deserialization
/// type, then wires them into this trait.
pub trait ConfigParser: Sized + Parser + ConfigFallback + CommandAware
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

        for arg in args.iter().cloned() {
            // short-circuit to have fully functional help/version output
            if let Some("--help" | "-h" | "--version" | "-V") = arg.into().as_os_str().to_str() {
                return Self::try_parse_from(args);
            }
        }

        let args = command.try_get_matches_from(args)?;
        let mut opts = Self::Opts::from_args(&args).unwrap_or_default();

        // Extract the command from opts before Figment merge so it is not subject to deep-merge
        // ordering issues with the config's multi-variant struct.
        let cli_command = Self::take_opts_command(&mut opts);

        let config: Option<Self::Config> = opts
            .config_path()
            .map(|path| {
                let path = PathBuf::from(path);

                if !path.exists() {
                    return Err(
                        Self::command().error(ErrorKind::Io, "configuration file not found")
                    );
                }

                let figment = match opts.config_format() {
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

                figment.extract::<Self::Config>().map_err(|e| {
                    Self::command().error(
                        ErrorKind::InvalidValue,
                        format!("invalid configuration file: {e}"),
                    )
                })
            })
            .transpose()?;

        let mut config = config;

        // Extract the command config before Figment merge for the same reason.
        let config_command = config
            .as_mut()
            .and_then(|c| Self::take_config_command(c))
            .unwrap_or_default();

        // Merge non-command fields: CLI values win over config defaults.
        let mut cli_figment = Figment::from(Serialized::defaults(&opts));

        if let Some(ref c) = config {
            cli_figment = cli_figment.join(Serialized::defaults(c));
        }

        let mut merged = cli_figment
            .extract::<Self::Opts>()
            .map_err(|e| Self::command().error(ErrorKind::InvalidValue, e.to_string()))?;

        // Resolve the command separately from the Figment merge.
        let resolved = config_command
            .merge_command(cli_command)
            .map_err(|msg| Self::command().error(ErrorKind::InvalidValue, msg))?;

        Self::put_opts_command(&mut merged, resolved);

        Self::try_parse_from(iter::once(command_name).chain(merged.into_args()))
    }
}
