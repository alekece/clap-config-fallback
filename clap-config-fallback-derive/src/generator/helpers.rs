use heck::{ToKebabCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ext::IdentExt};

use crate::{
    ClapArg, TypeExt,
    derive::{ClapCommand, ConfigPrecedence, NamedField},
    generator::GenerationTarget,
    syn_utils::IntoTokenStream,
};

pub(crate) fn generate_field_definition(
    ident: &Ident,
    field: &NamedField,
    field_prefix: Option<&Ident>,
    target: GenerationTarget,
) -> TokenStream {
    let field_ident = field.ident();

    if !field.commands().is_empty() {
        let field_ty = field.ty().unwrap_option();
        let target_ident = target.suffix_ident();
        let flatten_attr = field.flatten().then(|| quote! { #[serde(flatten)] });
        let field_attrs = match target {
            GenerationTarget::Opts => field.attributes().iter().into_token_stream(),
            GenerationTarget::Config => generate_serde_alias_attrs(field),
        };

        if field.flatten() && matches!(target, GenerationTarget::Config) {
            // serde turns any error inside a flattened `Option` into `None`, which would silently
            // drop invalid config instead of reporting it
            return quote! {
                #[serde(flatten, default)]
                #field_ident: <#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident
            };
        }

        return quote! {
            #field_attrs
            #flatten_attr
            #[serde(default)]
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            #field_ident: Option<<#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident>
        };
    }

    let field_ty = field.ty().to_option();
    let field_attrs = match target {
        GenerationTarget::Opts => {
            let sanitized_attrs = generate_sanitized_clap_attrs(field);
            let bool_attr = generate_clap_bool_action_attr(field);

            quote! {
                #sanitized_attrs
                #bool_attr
            }
        }
        GenerationTarget::Config => {
            let alias_attrs = generate_serde_alias_attrs(field);
            let deserialize_with_attr = generate_deserialize_with_attr(ident, field, field_prefix);

            quote! {
                #alias_attrs
                #deserialize_with_attr
            }
        }
    };

    quote! {
        #field_attrs
        #[serde(default)]
        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
        #field_ident: #field_ty
    }
}

pub(crate) fn generate_from_args_initializer(field: &NamedField) -> TokenStream {
    let field_ident = field.ident();
    let arg_id = generate_arg_id(field);

    if !field.commands().is_empty() {
        let field_ty = field.ty().unwrap_option();
        let target_ident = GenerationTarget::Opts.suffix_ident();

        quote! {
            #field_ident: <#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident::from_args(&args)
        }
    } else if field.ty().is("bool") {
        quote! {
            #field_ident: args.get_flag(#arg_id).then_some(true)
        }
    } else if field.ty().unwrap_option().is("Vec") {
        quote! {
            #field_ident: args
                .get_many(#arg_id)
                .map(|values| values.cloned().collect())
        }
    } else {
        quote! {
            #field_ident: args.get_one(#arg_id).cloned()
        }
    }
}

/// Returns the id clap gives the field's argument: the explicit `#[arg(id)]`, otherwise the field
/// name without its `r#` prefix.
fn generate_arg_id(field: &NamedField) -> String {
    field
        .args()
        .iter()
        .find_map(ClapArg::id)
        .unwrap_or_else(|| field.ident().unraw().to_string())
}

/// Returns the flag clap gives the field's argument, following clap's attribute order: an inherited
/// `short`/`long` uses the id set by any earlier `id`.
fn generate_flag_name(field: &NamedField) -> Option<String> {
    let mut name = field.ident().unraw().to_string().to_kebab_case();

    for arg in field.args() {
        if let Some(flag_name) = arg.flag_name(&name) {
            return Some(flag_name);
        }

        if let Some(id) = arg.id() {
            name = id;
        }
    }

    None
}

/// Generates the statement pushing a field's args.
///
/// Subcommand args go to `commands` so callers can append them after every other arg: clap
/// rejects parent args that follow a subcommand name. Other fields keep declaration order, which
/// clap uses to number positional args.
pub(crate) fn generate_extend_args_statement(
    ident: &Ident,
    commands_ident: &Ident,
    field: &NamedField,
) -> TokenStream {
    let field_ident = field.ident();

    if field.commands().contains(&ClapCommand::Subcommand) {
        quote! {
            ::clap_config_fallback::IntoArgs::extend_args(#field_ident, #commands_ident);
        }
    } else if !field.commands().is_empty() {
        quote! {
            ::clap_config_fallback::IntoArgs::extend_args_split(#field_ident, #ident, #commands_ident);
        }
    } else {
        let arg_name = generate_flag_name(field).map(|arg_name| quote! { .name(#arg_name) });

        let arg_formatter = field
            .value_format()
            .map(|formatter| quote! { .value_format(#formatter) });

        let arg_ctr = if field.ty().is("bool") {
            quote! { ::clap_config_fallback::Arg::flag(value) }
        } else if field.ty().unwrap_option().is("Vec") {
            quote! { ::clap_config_fallback::Arg::repeated(value) }
        } else {
            quote! { ::clap_config_fallback::Arg::scalar(value) }
        };

        quote! {
            #field_ident
                .map(|value| {
                    #arg_ctr
                        #arg_name
                        #arg_formatter
                }).extend_args(#ident);
        }
    }
}

/// Generates the statement pruning the config section of a command-like field to the branch
/// selected on the CLI.
///
/// `config` is a place holding the field's config struct; `opts` is an expression of type
/// `&Option<_>` for the field's CLI value.
pub(crate) fn generate_retain_commands_statement(
    config: &TokenStream,
    opts: &TokenStream,
    field: &NamedField,
) -> TokenStream {
    if field.commands().is_empty() || GenerationTarget::Config.should_skip(field) {
        return TokenStream::new();
    }

    let field_ident = field.ident();
    let field_ty = field.ty().unwrap_option();
    let retain_fn = quote! {
        <#field_ty as ::clap_config_fallback::ConfigFallback>::retain_commands
    };

    if field.commands().contains(&ClapCommand::Subcommand) {
        // no subcommand on the CLI: config must not select one
        quote! {
            match (&mut #config.#field_ident, #opts) {
                (
                    ::std::option::Option::Some(__clap_field_config),
                    ::std::option::Option::Some(__clap_field_opts),
                ) => #retain_fn(__clap_field_config, __clap_field_opts),
                (__clap_field_config, _) => *__clap_field_config = ::std::option::Option::None,
            }
        }
    } else if field.flatten() {
        // flattened args without CLI values may still hold subcommands to prune
        quote! {
            match #opts {
                ::std::option::Option::Some(__clap_field_opts) => {
                    #retain_fn(&mut #config.#field_ident, __clap_field_opts)
                }
                ::std::option::Option::None => {
                    #retain_fn(&mut #config.#field_ident, &::std::default::Default::default())
                }
            }
        }
    } else {
        quote! {
            if let ::std::option::Option::Some(__clap_field_config) = &mut #config.#field_ident {
                match #opts {
                    ::std::option::Option::Some(__clap_field_opts) => {
                        #retain_fn(__clap_field_config, __clap_field_opts)
                    }
                    ::std::option::Option::None => {
                        #retain_fn(__clap_field_config, &::std::default::Default::default())
                    }
                }
            }
        }
    }
}

pub(crate) fn generate_deserialize_fn(
    field: &NamedField,
    field_prefix: Option<&Ident>,
) -> TokenStream {
    let Some(parser) = field.args().iter().find_map(ClapArg::value_parser) else {
        return TokenStream::new();
    };

    let field_ty = field.ty().to_option();
    let fn_ident = if let Some(field_prefix) = field_prefix {
        format_ident!(
            "deserialize_{}_{}",
            field_prefix.to_string().to_snake_case(),
            field.ident()
        )
    } else {
        format_ident!("deserialize_{}", field.ident())
    };

    let deserialize_body = if field.ty().unwrap_option().is("Vec") {
        quote! {
            let s: ::std::option::Option<::std::vec::Vec<String>> = ::serde::Deserialize::deserialize(deserializer)?;

            s
                .map(|s| s
                    .into_iter()
                    .map(|s| #parser(s.as_str()).map_err(::serde::de::Error::custom))
                    .collect::<::std::result::Result<::std::vec::Vec<_>, _>>()
                )
                .transpose()

        }
    } else {
        quote! {
            let s: ::std::option::Option<String> = ::serde::Deserialize::deserialize(deserializer)?;

            s.map(|s| #parser(s.as_str()).map_err(::serde::de::Error::custom)).transpose()
        }
    };

    quote! {
        fn #fn_ident<'de, D>(deserializer: D) -> ::std::result::Result<#field_ty, D::Error>
        where
            D: ::serde::de::Deserializer<'de>,
        {
            #deserialize_body
        }
    }
}

pub(crate) fn generate_deserialize_with_attr(
    ident: &Ident,
    field: &NamedField,
    field_prefix: Option<&Ident>,
) -> TokenStream {
    field
        .args()
        .iter()
        .find_map(|arg| arg.value_parser())
        .map_or(TokenStream::new(), |_| {
            let field_ident = field.ident();
            let deserialize_fn = if let Some(field_prefix) = field_prefix {
                format!(
                    "{}::deserialize_{}_{}",
                    ident,
                    field_prefix.to_string().to_snake_case(),
                    field_ident
                )
            } else {
                format!("{}::deserialize_{}", ident, field_ident)
            };

            quote! { #[serde(deserialize_with = #deserialize_fn)] }
        })
}

pub(crate) fn generate_serde_alias_attrs(field: &NamedField) -> TokenStream {
    let mut aliases: Vec<String> = if !field.commands().is_empty() {
        // `clap` does not support aliases for command-like, so we forward any
        // `#[config(alias, aliases)]` directly
        field
            .aliases()
            .into_iter()
            .map(|alias| alias.value())
            .collect()
    } else {
        field
            .args()
            .iter()
            .flat_map(ClapArg::aliases)
            .map(|alias| alias.value())
            .collect()
    };

    // Config files may spell multi-word keys in kebab-case; the canonical key stays the field name
    // so the config merges with the generated `Opts`.
    let field_name = field.ident().unraw().to_string();
    let kebab_name = field_name.to_kebab_case();

    if !field.flatten() && kebab_name != field_name && !aliases.contains(&kebab_name) {
        aliases.push(kebab_name);
    }

    aliases
        .iter()
        .map(|alias| quote! { #[serde(alias = #alias)] })
        .into_token_stream()
}

pub(crate) fn generate_sanitized_clap_attrs(field: &NamedField) -> TokenStream {
    field
        .attributes()
        .iter()
        .map(|attr| {
            let denied_args: &[&str] = match (field.precedence(), field.is_path()) {
                (ConfigPrecedence::AfterDefault, false) | (_, true) => {
                    &["require", "conflicts", "exclusive"]
                }
                (ConfigPrecedence::BeforeDefault, false) => {
                    &["default", "require", "conflicts", "exclusive"]
                }
                (ConfigPrecedence::BeforeEnv, false) => {
                    &["default", "env", "require", "conflicts", "exclusive"]
                }
            };

            ClapArg::sanitize(attr, denied_args)
        })
        .into_token_stream()
}

pub(crate) fn generate_clap_bool_action_attr(field: &NamedField) -> TokenStream {
    if field.ty().is("bool") {
        quote! { #[arg(action = clap::ArgAction::SetTrue)] }
    } else {
        TokenStream::new()
    }
}
