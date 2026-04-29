use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{derive::NamedField, generator::GenerationTarget, ClapArg, ClapCommand, TypeExt};

pub(crate) fn generate_field_definition(
    ident: &Ident,
    field: &NamedField,
    field_prefix: Option<&Ident>,
    target: GenerationTarget,
) -> TokenStream {
    let field_ident = field.ident();

    if let Some(field_attr) = field
        .attributes()
        .find(|attr| ClapCommand::from_attr(attr).is_some())
    {
        let field_ty = format_ident!(
            "{}{}",
            field.ty().unwrap_option().ident().unwrap(),
            target.suffix()
        );

        return match target {
            GenerationTarget::Opts => quote! {
                #field_attr
                #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                #field_ident: Option<#field_ty>
            },
            GenerationTarget::Config => quote! {
                #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                #field_ident: ::std::option::Option<#field_ty>
            },
        };
    }

    let field_ty = field.ty().to_option();
    let field_attrs = match target {
        GenerationTarget::Opts => {
            let attrs = field.attributes().cloned().map(ClapArg::sanitize);
            let bool_attr = field
                .ty()
                .is("bool")
                .then(|| quote! { #[arg(action = clap::ArgAction::SetTrue)] });

            quote! {
                #(#attrs)*
                #bool_attr
            }
        }
        GenerationTarget::Config => {
            let alias_attrs = field
                .attributes()
                .filter_map(ClapArg::from_attr)
                .flat_map(|arg| arg.aliases())
                .map(|alias| quote! { #[serde(alias = #alias)] });

            let deserialize_attr = field
                .attributes()
                .filter_map(ClapArg::from_attr)
                .find_map(|arg| arg.value_parser())
                .map(|_| {
                    let deserialize_fn = if let Some(field_prefix) = field_prefix {
                        format!("{}::deserialize_{}_{}", ident, field_prefix, field_ident)
                    } else {
                        format!("{}::deserialize_{}", ident, field_ident)
                    };

                    quote! { #[serde(deserialize_with = #deserialize_fn)] }
                });

            quote! {
                #(#alias_attrs)*
                #deserialize_attr
            }
        }
    };

    quote! {
        #field_attrs
        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
        #field_ident: #field_ty
    }
}

pub(crate) fn generate_from_args_initializer(
    field: &NamedField,
    field_suffix: &str,
) -> TokenStream {
    let field_ident = field.ident();

    if field
        .attributes()
        .find_map(ClapCommand::from_attr)
        .is_some()
    {
        let ty_ident = format_ident!(
            "{}{}",
            field.ty().unwrap_option().ident().unwrap(),
            field_suffix
        );

        quote! {
            #field_ident: #ty_ident::from_args(&args)
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

    if field
        .attributes()
        .find_map(ClapCommand::from_attr)
        .is_some()
    {
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
            .attributes()
            .filter_map(ClapArg::from_attr)
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
    let Some(parser) = field
        .attributes()
        .filter_map(ClapArg::from_attr)
        .find_map(|arg| arg.value_parser())
    else {
        return TokenStream::new();
    };

    let field_ty = field.ty().to_option();
    let fn_ident = if let Some(field_prefix) = field_prefix {
        format_ident!("deserialize_{}_{}", field_prefix, field.ident())
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
