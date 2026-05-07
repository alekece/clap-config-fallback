use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{
    ClapArg, TypeExt,
    derive::{ConfigPrecedence, NamedField},
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

    if !field.commands().is_empty() {
        let field_ty = field.ty().unwrap_option();
        let target_ident = GenerationTarget::Opts.suffix_ident();

        quote! {
            #field_ident: <#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident::from_args(&args)
        }
    } else if field.ty().is("bool") {
        quote! {
            #field_ident: args.get_flag(stringify!(#field_ident)).then_some(true)
        }
    } else if field.ty().is("Vec") {
        quote! {
            #field_ident: args
                .get_many(stringify!(#field_ident))
                .map(|values| values.cloned().collect())
        }
    } else {
        quote! {
            #field_ident: args.get_one(stringify!(#field_ident)).cloned()
        }
    }
}

pub(crate) fn generate_into_args_statement(ident: &Ident, field: &NamedField) -> TokenStream {
    let field_ident = field.ident();

    if !field.commands().is_empty() {
        quote! {
            if let Some(#field_ident) = #field_ident {
                 #ident.extend(#field_ident.into_args());
            }
        }
    } else {
        let formatted_value = field
            .value_format()
            .map(|formatter| quote! { (#formatter)(value).to_string() })
            .unwrap_or_else(|| quote! { value.to_string() });

        let flag_name = field
            .args()
            .iter()
            .find_map(|arg| arg.flag_name(field_ident))
            .map(|flag_name| quote! { #flag_name.to_string() });

        match flag_name {
            Some(flag_name) if field.ty().is("bool") => quote! {
                if let Some(true) = #field_ident {
                    #ident.push(#flag_name);
                }
            },
            Some(flag_name) if field.ty().is("Vec") => quote! {
                if let Some(values) = #field_ident {
                    for value in values {
                        #ident.push(#flag_name);
                        #ident.push(#formatted_value);
                    }
                }
            },
            Some(flag_name) => quote! {
                if let Some(value) = #field_ident {
                    #ident.push(#flag_name);
                    #ident.push(#formatted_value);
                }
            },
            None if field.ty().is("Vec") => quote! {
                if let Some(values) = #field_ident {
                    for value in values {
                        #ident.push(#formatted_value);
                    }
                }
            },
            None => quote! {
                if let Some(value) = #field_ident {
                    #ident.push(#formatted_value);
                }
            },
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

    quote! {
        fn #fn_ident<'de, D>(deserializer: D) -> ::std::result::Result<#field_ty, D::Error>
        where
            D: ::serde::de::Deserializer<'de>,
        {
            let s: ::std::option::Option<String> = ::serde::Deserialize::deserialize(deserializer)?;

            s.map(|s| #parser(s.as_str()).map_err(::serde::de::Error::custom)).transpose()
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
    if !field.commands().is_empty() {
        // `clap` does not support aliases for command-like, so we forward any
        // `#[config(alias, aliases)]` directly
        field
            .aliases()
            .into_iter()
            .map(|alias| quote! { #[serde(alias = #alias)] })
            .into_token_stream()
    } else {
        field
            .args()
            .iter()
            .flat_map(ClapArg::aliases)
            .map(|alias| quote! { #[serde(alias = #alias)] })
            .into_token_stream()
    }
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
