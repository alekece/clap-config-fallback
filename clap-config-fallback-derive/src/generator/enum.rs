use heck::ToKebabCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{
    derive::{ConfigSubcommand, Variant, VariantShape},
    generator::{GenerationTarget, helpers},
    syn_utils::IntoTokenStream,
};

/// Common interface for parsed derive input that behave like enums.
pub trait EnumLike {
    /// Identifier of the enum.
    fn ident(&self) -> &Ident;
    /// Variant of the enum.
    fn variants(&self) -> &[Variant];
}

/// Generatates `Opts` and `Config` helper enums and impls for derive inputs.
pub struct EnumGenerator<T: EnumLike> {
    input: T,
}

impl EnumGenerator<ConfigSubcommand> {
    pub fn generate(&self) -> TokenStream {
        let ident = self.input.ident();
        let (opts_ident, opts) = self.generate_opts_enum();
        let (config_ident, config) = self.generate_config_struct();
        let deserialize_fns = self.generate_deserialize_fns();
        let extend_arg_fn = self.generate_extend_args_fn();
        let from_args_fn = self.generate_from_args_fn();
        let retain_commands_fn = self.generate_retain_commands_fn(&opts_ident);

        quote! {
            #[derive(::clap::Subcommand)]
            #opts

            #config

            impl #config_ident {
                #deserialize_fns
            }

            impl ::clap_config_fallback::IntoArgs for #opts_ident {
                #extend_arg_fn
            }

            impl ::clap_config_fallback::FromArgs for #opts_ident {
                #from_args_fn
            }

            impl ::clap_config_fallback::ConfigFallback for #ident {
                type Opts = #opts_ident;
                type Config = #config_ident;

                #retain_commands_fn
            }
        }
    }
}

impl<T: EnumLike> EnumGenerator<T> {
    /// Creates a new `EnumGenerator` for the given derive input.
    pub fn new(input: T) -> Self {
        Self { input }
    }

    fn generate_opts_enum(&self) -> (Ident, TokenStream) {
        let target = GenerationTarget::Opts;
        let ident = format_ident!("{}{}", self.input.ident(), target.suffix());
        let variants = self.input.variants().iter().map(|variant| {
            let variant_ident = variant.ident();
            // must match the config section name so both sides merge
            let variant_name = variant_ident.to_string().to_kebab_case();
            let variant = match variant.shape() {
                VariantShape::Unit => quote! { #variant_ident },
                VariantShape::Newtype(ty) => {
                    let target_ident = target.suffix_ident();

                    quote! { #variant_ident(<#ty as ::clap_config_fallback::ConfigFallback>::#target_ident) }
                }
                VariantShape::Struct(fields) => {
                    let fields = fields.iter().map(|field| {
                        helpers::generate_field_definition(
                            &ident,
                            field,
                            Some(variant_ident),
                            target,
                        )
                    });

                    quote! { #variant_ident { #(#fields),* } }
                }
            };

            quote! {
                #[serde(rename = #variant_name)]
                #variant
            }
        });

        (
            ident.clone(),
            quote! {
                #[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
                pub enum #ident {
                    #(#variants,)*
                }
            },
        )
    }

    /// Generates the config type: one optional section per variant, so a config file can hold
    /// parameters for every subcommand at once without selecting any of them.
    fn generate_config_struct(&self) -> (Ident, TokenStream) {
        let target = GenerationTarget::Config;
        let ident = format_ident!("{}{}", self.input.ident(), target.suffix());
        let (section_structs, sections): (Vec<_>, Vec<_>) = self
            .input
            .variants()
            .iter()
            .filter(|variant| !target.should_skip(*variant))
            .map(|variant| {
                let variant_ident = variant.ident();
                let section_ident = format_ident!("{}{}", ident, variant_ident);
                let section_name = variant_ident.to_string().to_kebab_case();
                let section_struct = |fields: TokenStream| {
                    quote! {
                        #[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]
                        pub struct #section_ident {
                            #fields
                        }
                    }
                };

                let (section_struct, section_ty) = match variant.shape() {
                    VariantShape::Unit => (
                        section_struct(TokenStream::new()),
                        quote! { #section_ident },
                    ),
                    VariantShape::Newtype(ty) => (
                        TokenStream::new(),
                        quote! { <#ty as ::clap_config_fallback::ConfigFallback>::Config },
                    ),
                    VariantShape::Struct(fields) => {
                        let fields = fields
                            .iter()
                            .filter(|field| !target.should_skip(*field))
                            .map(|field| {
                                helpers::generate_field_definition(
                                    &ident,
                                    field,
                                    Some(variant_ident),
                                    target,
                                )
                            });

                        (
                            section_struct(quote! { #(#fields),* }),
                            quote! { #section_ident },
                        )
                    }
                };

                (
                    section_struct,
                    quote! {
                        #[serde(rename = #section_name)]
                        #[serde(default)]
                        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                        #variant_ident: ::std::option::Option<#section_ty>
                    },
                )
            })
            .unzip();

        (
            ident.clone(),
            quote! {
                #(#section_structs)*

                #[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]
                #[allow(non_snake_case)]
                #[serde(deny_unknown_fields)]
                pub struct #ident {
                    #(#sections,)*
                }
            },
        )
    }

    /// Generates `ConfigFallback::retain_commands`, keeping only the section of the
    /// variant selected on the CLI and recursing into its nested subcommands.
    fn generate_retain_commands_fn(&self, opts_ident: &Ident) -> TokenStream {
        let config = format_ident!("__clap_config");
        let sections = format_ident!("__clap_sections");
        let section = format_ident!("__clap_section");
        let arms = self.input.variants().iter().map(|variant| {
            let variant_ident = variant.ident();

            if GenerationTarget::Config.should_skip(variant) {
                let pattern = match variant.shape() {
                    VariantShape::Unit => quote! { #opts_ident::#variant_ident },
                    VariantShape::Newtype(_) => quote! { #opts_ident::#variant_ident(_) },
                    VariantShape::Struct(_) => quote! { #opts_ident::#variant_ident { .. } },
                };

                return quote! { #pattern => {} };
            }

            match variant.shape() {
                // unit variants have no parameters to merge
                VariantShape::Unit => quote! { #opts_ident::#variant_ident => {} },
                VariantShape::Newtype(ty) => quote! {
                    #opts_ident::#variant_ident(__clap_opts) => {
                        if let ::std::option::Option::Some(mut #section) = #sections.#variant_ident {
                            <#ty as ::clap_config_fallback::ConfigFallback>::retain_commands(
                                &mut #section,
                                __clap_opts,
                            );
                            #config.#variant_ident = ::std::option::Option::Some(#section);
                        }
                    }
                },
                VariantShape::Struct(fields) => {
                    let command_fields = fields
                        .iter()
                        .filter(|field| {
                            !field.commands().is_empty()
                                && !GenerationTarget::Config.should_skip(*field)
                        })
                        .collect::<Vec<_>>();
                    let bindings = command_fields.iter().map(|field| {
                        let field_ident = field.ident();
                        let binding = format_ident!("__clap_field_{}", field_ident);

                        quote! { #field_ident: #binding }
                    });
                    let statements = command_fields.iter().map(|field| {
                        let binding = format_ident!("__clap_field_{}", field.ident());

                        helpers::generate_retain_commands_statement(
                            &quote! { #section },
                            &quote! { #binding },
                            field,
                        )
                    });

                    quote! {
                        #opts_ident::#variant_ident { #(#bindings,)* .. } => {
                            if let ::std::option::Option::Some(mut #section) = #sections.#variant_ident {
                                #(#statements)*
                                #config.#variant_ident = ::std::option::Option::Some(#section);
                            }
                        }
                    }
                }
            }
        });

        quote! {
            #[allow(unused_mut, unused_variables)]
            fn retain_commands(#config: &mut Self::Config, __clap_opts: &Self::Opts) {
                let #sections = ::std::mem::take(#config);

                match __clap_opts {
                    #(#arms,)*
                }
            }
        }
    }

    fn generate_extend_args_fn(&self) -> TokenStream {
        let ident = format_ident!("__clap_args");
        let variant_matches = self.input.variants().iter().map(|variant| {
            let variant_ident = variant.ident();
            let formatted_variant = variant_ident.to_string().to_kebab_case();
            let variant_arg = quote! {
                ::clap_config_fallback::Arg::scalar(#formatted_variant).extend_args(#ident)
            };

            match variant.shape() {
                VariantShape::Unit => quote! {
                    Self::#variant_ident => #variant_arg
                },
                VariantShape::Newtype(_) => quote! {
                    Self::#variant_ident(value) => {
                        #variant_arg;
                        value.extend_args(#ident);
                    }
                },
                VariantShape::Struct(fields) => {
                    let (field_idents, field_statements): (Vec<_>, Vec<_>) = fields
                        .iter()
                        .map(|field| {
                            (
                                field.ident(),
                                helpers::generate_extend_args_statement(&ident, field),
                            )
                        })
                        .unzip();

                    quote! {
                        Self::#variant_ident { #(#field_idents,)* } => {
                            #variant_arg;

                            #(#field_statements)*
                        }
                    }
                }
            }
        });

        quote! {
            fn extend_args(self, #ident: &mut ::std::vec::Vec<::std::ffi::OsString>) {
                match self {
                    #(#variant_matches,)*
                }
            }
        }
    }

    fn generate_from_args_fn(&self) -> TokenStream {
        let variant_matches = self.input.variants().iter().map(|variant| {
            let ident = variant.ident();
            let formatted_variant = ident.to_string().to_kebab_case();

            match variant.shape() {
                VariantShape::Unit => quote! {
                    ::std::option::Option::Some((#formatted_variant, _)) =>
                        ::std::option::Option::Some(Self::#ident)
                },
                VariantShape::Newtype(ty) => {
                    let target_ident = GenerationTarget::Opts.suffix_ident();

                    // the variant was selected on the CLI even when all its values come from config
                    quote! {
                        ::std::option::Option::Some((#formatted_variant, args)) =>
                            ::std::option::Option::Some(Self::#ident(
                                <#ty as ::clap_config_fallback::ConfigFallback>::#target_ident::from_args(args)
                                    .unwrap_or_default()
                            ))
                    }
                }
                VariantShape::Struct(fields) => {
                    let field_assignments =
                        fields.iter().map(helpers::generate_from_args_initializer);

                    quote! {
                        ::std::option::Option::Some((#formatted_variant, args)) =>
                            ::std::option::Option::Some(Self::#ident { #(#field_assignments),* })
                    }
                }
            }
        });

        quote! {
            fn from_args(args: &::clap::ArgMatches) -> Option<Self> {
                match args.subcommand() {
                    #(#variant_matches,)*
                    _ => None,
                }
            }
        }
    }

    fn generate_deserialize_fns(&self) -> TokenStream {
        self.input
            .variants()
            .iter()
            .filter(|variant| !GenerationTarget::Config.should_skip(*variant))
            .filter_map(|variant| {
                variant
                    .as_struct()
                    .map(|fields| fields.into_iter().map(|field| (variant.ident(), field)))
            })
            .flatten()
            .filter(|(_, field)| !GenerationTarget::Config.should_skip(field))
            .map(|(ident, field)| helpers::generate_deserialize_fn(&field, Some(ident)))
            .into_token_stream()
    }
}
