mod clap;
mod config_parser;
mod syn_utils;

use darling::FromDeriveInput;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Ident, parse_macro_input};

use clap::{ClapArg, ClapCommand};
use config_parser::{ConfigFormat, ConfigParser};
use syn_utils::TypeExt;

/// Derive macro for `ConfigParser`, which generates the necessary code to implement the
/// `ConfigParser` trait for a given struct, allowing it to be parsed from both CLI arguments and a
/// configuration file, with support for nested commands and various configuration formats.
#[proc_macro_derive(ConfigParser, attributes(config))]
pub fn derive_parser_config(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let config_parser = match ConfigParser::from_derive_input(&input) {
        Ok(v) => v,
        Err(e) => return proc_macro::TokenStream::from(e.write_errors()),
    };

    let ident = config_parser.ident();
    let config_path_fn = generate_config_path_fn(&config_parser);
    let config_format_fn = generate_config_format_fn(&config_parser);
    let into_args_fn = generate_into_args_fn(&config_parser);
    let from_args_fn = generate_from_args_fn(&config_parser);
    let deserialize_fns = generate_deserialize_fns(&config_parser);
    let (config_ident, config_struct) = generate_config_struct(&config_parser);
    let (opts_ident, opts_struct) = generate_opts_struct(&config_parser);

    quote! {
        #config_struct
        #opts_struct

        impl #config_ident {
            #deserialize_fns
        }

        impl ::clap_config_fallback::IntoArgs for #opts_ident {
            #into_args_fn
        }

        impl ::clap_config_fallback::FromArgs for #opts_ident {
            #from_args_fn
        }

        impl ::clap_config_fallback::ConfigSource for #opts_ident {
            #config_path_fn
            #config_format_fn
        }

        impl ::clap_config_fallback::ConfigParser for #ident {
            type Opts = #opts_ident;
            type Config = #config_ident;
        }
    }
    .into()
}

/// Generates the `from_matches` method for the `Opts` struct.
fn generate_from_args_fn(config_parser: &ConfigParser) -> TokenStream {
    let field_assignments = config_parser.fields().map(|field| {
        let field_ident = field.ident();

        if field
            .attributes()
            .find_map(ClapCommand::from_attr)
            .is_some()
        {
            let opts_ident = format_ident!("{}Opts", field.ty().ident().unwrap());

            quote! {
                #field_ident: #opts_ident::from_args(&args)
            }
        } else if field.ty().is("bool") {
            quote! {
                #field_ident: args.get_flag(stringify!(#field_ident)).then_some(true)
            }
        } else {
            quote! {
                #field_ident: args.get_one(stringify!(#field_ident)).cloned()
            }
        }
    });

    quote! {
        fn from_args(args: &::clap::ArgMatches) -> Self {
            Self {
                #(#field_assignments),*
            }
        }
    }
}

/// Generates the `config_format` method for the `Opts` struct, which returns the format of the
/// configuration file if specified.
fn generate_config_format_fn(config_parser: &ConfigParser) -> TokenStream {
    let format = match config_parser
        .fields()
        .find(|field| field.is_path())
        .and_then(|field| field.format())
    {
        Some(ConfigFormat::Toml) => format_ident!("Toml"),
        Some(ConfigFormat::Yaml) => format_ident!("Yaml"),
        Some(ConfigFormat::Json) => format_ident!("Json"),
        // do not override the default `config_format` implementation if no format is specified or
        // if the format is set to `ConfigFormat::Auto`.
        None | Some(ConfigFormat::Auto) => return TokenStream::default(),
    };

    quote! {
        fn config_format(&self) -> ::std::option::Option<::clap_config_fallback::ConfigFormat> {
            ::std::option::Option::Some(::clap_config_fallback::ConfigFormat::#format)
        }
    }
}

/// Generates the `config_path` method for the `Opts` struct, which returns the path to the
/// configuration file if specified, either from the CLI argument or from the default value
/// specified in the original struct's field attributes.
fn generate_config_path_fn(config_parser: &ConfigParser) -> TokenStream {
    let Some(field) = config_parser.fields().find(|field| field.is_path()) else {
        return quote! {
            fn config_path(&self) -> ::std::option::Option<&str> {
                ::std::option::Option::None
            }
        };
    };

    let field_ident = field.ident();
    let default_path = field
        .attributes()
        .filter_map(ClapArg::from_attr)
        .find_map(|arg| arg.default_value().map(str::to_owned));

    if let Some(default_path) = default_path {
        quote! {
            fn config_path(&self) -> ::std::option::Option<&str> {
                self.#field_ident.as_deref().or(Some(#default_path))
            }
        }
    } else {
        quote! {
            fn config_path(&self) -> ::std::option::Option<&str> {
                self.#field_ident.as_deref()
            }
        }
    }
}

/// Generates the `into_args` method for the `Opts` struct, which converts the parsed options into a
/// vector of strings that can be used as CLI arguments for the original struct's `Parser`
/// implementation.
fn generate_into_args_fn(config_parser: &ConfigParser) -> TokenStream {
    let args = config_parser.fields().map(|field| {
        let field_ident = field.ident();

        if field
            .attributes()
            .find_map(ClapCommand::from_attr)
            .is_some()
        {
            quote! {
                args.extend(self.#field_ident.into_args());
            }
        } else {
            let formatted_value = field
                .value_format()
                .map(|value_format| quote! { #value_format })
                .unwrap_or_else(|| quote! { #field_ident.to_string() });

            match field
                .attributes()
                .filter_map(ClapArg::from_attr)
                .find_map(|arg| arg.flag_name(field_ident))
            {
                Some(flag_name) if field.ty().is("bool") => {
                    quote! {
                        if let Some(true) = self.#field_ident {
                            args.push(#flag_name.to_string());
                        }
                    }
                }
                Some(flag_name) => {
                    quote! {
                        if let Some(#field_ident) = self.#field_ident {
                            args.push(#flag_name.to_string());
                            args.push(#formatted_value);
                        }
                    }
                }
                None => {
                    quote! {
                        if let Some(#field_ident) = self.#field_ident {
                            args.push(#formatted_value);
                        }
                    }
                }
            }
        }
    });

    quote! {
        fn into_args(self) -> impl Iterator<Item = String> {
            let mut args = Vec::default();

            #(#args)*

            args.into_iter()
        }
    }
}

fn generate_deserialize_fns(config_parser: &ConfigParser) -> TokenStream {
    let deserialize_fns = config_parser.fields().filter(|field| !field.is_skipped()).filter_map(|field| {
        if let Some(path) = field
            .attributes()
            .filter_map(ClapArg::from_attr)
            .find_map(|arg| arg.value_parser())
        {
            let fn_ident = format_ident!("deserialize_{}", field.ident());
            let field_ty = field.ty().to_option();

            Some(quote! {
                fn #fn_ident<'de, D>(deserializer: D) -> ::std::result::Result<#field_ty, D::Error>
                where
                    D: ::serde::de::Deserializer<'de>,
                {
                    let s: ::std::option::Option<String> = ::serde::Deserialize::deserialize(deserializer)?;

                    s
                        .map(|s| #path(s.as_str()).map_err(::serde::de::Error::custom))
                        .transpose()
                }
            })
        } else {
            None
        }
    });

    quote! {
        #(#deserialize_fns)*
    }
}

/// Generates the configuration struct which is used to serialize and deserialize the configuration
/// file.
///
/// This struct will have the same fields as the original struct, but all fields will be optional
/// and will have the appropriate `serde` attributes for serialization and deserialization.
fn generate_config_struct(config_parser: &ConfigParser) -> (Ident, TokenStream) {
    let ident = format_ident!("{}Config", config_parser.ident());

    let fields = config_parser
        .fields()
        .filter(|field| !field.is_skipped())
        .map(|field| {
            let field_ident = field.ident();
            let clap_attrs = field
                .attributes()
                .filter_map(ClapArg::from_attr)
                .collect::<Vec<_>>();
            let alias_attrs = clap_attrs
                .iter()
                .flat_map(|arg| arg.aliases())
                .map(|alias| quote! { #[serde(alias = #alias)] });
            let deserialize_with_attr =
                clap_attrs
                    .iter()
                    .find_map(|arg| arg.value_parser())
                    .map(|_| {
                        let deserialize_fn = format!("{}::deserialize_{}", ident, field_ident);

                        quote! { #[serde(deserialize_with = #deserialize_fn)] }
                    });

            if field
                .attributes()
                .find_map(ClapCommand::from_attr)
                .is_some()
            {
                let field_ty = format_ident!("{}Config", field.ty().ident().unwrap());

                quote! {
                    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                    #field_ident: ::std::option::Option<#field_ty>
                }
            } else {
                let field_ty = &field.ty().to_option();

                quote! {
                    #(#alias_attrs)*
                    #deserialize_with_attr
                    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                    #field_ident: #field_ty
                }
            }
        });

    (
        ident.clone(),
        quote! {
            #[derive(Debug, serde::Serialize, serde::Deserialize)]
            struct #ident {
                #(#fields),*
            }
        },
    )
}

/// Generates the `Opts` struct which is used to parse CLI arguments.
///
/// This struct will have the same fields as the original struct, but all fields will be optional
/// and will have the appropriate `clap` attributes for parsing.
/// For fields that are marked with `#[command]`, the corresponding `Opts` struct will be used as
/// the field type.
fn generate_opts_struct(config_parser: &ConfigParser) -> (Ident, TokenStream) {
    let ident = format_ident!("{}Opts", config_parser.ident());

    let fields = config_parser.fields().map(|field| {
        let field_ident = field.ident();

        if field
            .attributes()
            .find_map(ClapCommand::from_attr)
            .is_some()
        {
            let field_ty = format_ident!("{}Opts", field.ty().ident().unwrap());

            return quote! {
                #[command(flatten)]
                #field_ident: #field_ty
            };
        }

        let field_attrs = field.attributes().cloned().map(ClapArg::sanitize);
        let field_ty = field.ty().to_option();

        if field.ty().is("bool") {
            quote! {
                #(#field_attrs)*
                #[arg(action = clap::ArgAction::SetTrue)]
                #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                #field_ident: #field_ty
            }
        } else {
            quote! {
                #(#field_attrs)*
                #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                #field_ident: #field_ty
            }
        }
    });

    (
        ident.clone(),
        quote! {
            #[derive(Debug, clap::Parser, serde::Serialize, serde::Deserialize)]
            struct #ident {
                #(#fields),*
            }
        },
    )
}
