use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{ClapArg, TypeExt, derive::NamedField, generator::GenerationTarget};

pub(crate) fn generate_field_definition(
    ident: &Ident,
    field: &NamedField,
    field_prefix: Option<&Ident>,
    target: GenerationTarget,
) -> TokenStream {
    let field_ident = field.ident();

    if !field.commands().is_empty() {
        let field_ty = field.ty();
        let target_ident = target.suffix_ident();

        return match target {
            GenerationTarget::Opts => {
                let field_attrs = field.attributes();

                quote! {
                    #(#field_attrs)*
                    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                    #field_ident: Option<<#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident>
                }
            }
            GenerationTarget::Config => {
                let alias_attrs = field
                    .aliases()
                    .into_iter()
                    .map(|alias| quote! { #[serde(alias = #alias)] });

                quote! {
                    #(#alias_attrs)*
                    #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                    #field_ident: ::std::option::Option<<#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident>
                }
            }
        };
    }

    let field_ty = field.ty().to_option();
    let field_attrs = match target {
        GenerationTarget::Opts => {
            let sanitized_attrs = field.attributes().iter().cloned().map(ClapArg::sanitize);
            let bool_attr = field
                .ty()
                .is("bool")
                .then(|| quote! { #[arg(action = clap::ArgAction::SetTrue)] });

            quote! {
                #(#sanitized_attrs)*
                #bool_attr
            }
        }
        GenerationTarget::Config => {
            let alias_attrs = field
                .args()
                .iter()
                .flat_map(ClapArg::aliases)
                .map(|alias| quote! { #[serde(alias = #alias)] });

            let deserialize_attr =
                field
                    .args()
                    .iter()
                    .find_map(|arg| arg.value_parser())
                    .map(|_| {
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
                    });

            quote! {
                #(#alias_attrs)*
                #deserialize_attr
                #[serde(default)]
            }
        }
    };

    quote! {
        #field_attrs
        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
        #field_ident: #field_ty
    }
}

pub(crate) fn generate_from_args_initializer(field: &NamedField) -> TokenStream {
    let field_ident = field.ident();

    if !field.commands().is_empty() {
        let field_ty = field.ty();
        let target_ident = GenerationTarget::Opts.suffix_ident();

        quote! {
            #field_ident: <#field_ty as ::clap_config_fallback::ConfigFallback>::#target_ident::from_args(&args)
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
            .map(|value_format| quote! { #value_format })
            .unwrap_or_else(|| quote! { #field_ident.to_string() });

        match field
            .args()
            .iter()
            .find_map(|arg| arg.flag_name(field_ident))
        {
            Some(flag_name) if field.ty().is("bool") => {
                quote! {
                    if let Some(true) = #field_ident {
                        #ident.push(#flag_name.to_string());
                    }
                }
            }
            Some(flag_name) => {
                quote! {
                    if let Some(#field_ident) = #field_ident {
                        #ident.push(#flag_name.to_string());
                        #ident.push(#formatted_value);
                    }
                }
            }
            None => {
                quote! {
                    if let Some(#field_ident) = #field_ident {
                        #ident.push(#formatted_value);
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
