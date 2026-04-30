//! Proc-macro implementation for `clap_config_fallback` derives.

mod derive;
mod generator;
mod syn_utils;

use darling::FromDeriveInput;
use syn::{DeriveInput, parse_macro_input};

use self::{
    derive::{ClapArg, ConfigArgs, ConfigParser, ConfigSubcommand},
    generator::{EnumGenerator, StructGenerator},
    syn_utils::TypeExt,
};

/// Derives `clap_config_fallback::ConfigParser` for a clap `Parser` root struct.
#[proc_macro_derive(ConfigParser, attributes(config))]
pub fn derive_config_parse(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    ConfigParser::from_derive_input(&input)
        .map(|v| StructGenerator::new(v).generate())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

/// Derives `clap_config_fallback::ConfigArgs` for clap `Args` structs.
#[proc_macro_derive(ConfigArgs, attributes(config))]
pub fn derive_config_args(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    ConfigArgs::from_derive_input(&input)
        .map(|v| StructGenerator::new(v).generate())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}

/// Derives `clap_config_fallback::ConfigSubcommand` for clap `Subcommand` enums.
#[proc_macro_derive(ConfigSubcommand, attributes(config))]
pub fn derive_config_subcommand(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    ConfigSubcommand::from_derive_input(&input)
        .map(|v| EnumGenerator::new(v).generate())
        .unwrap_or_else(|e| e.write_errors())
        .into()
}
