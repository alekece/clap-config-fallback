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
    /// Tag to use for internally tagged enums.
    fn tag(&self) -> Option<&str>;
}

/// Generatates `Opts` and `Config` helper enums and impls for derive inputs.
pub struct EnumGenerator<T: EnumLike> {
    input: T,
}

impl EnumGenerator<ConfigSubcommand> {
    pub fn generate(&self) -> TokenStream {
        let ident = self.input.ident();
        let (opts_ident, opts) = self.generate_enum(GenerationTarget::Opts);
        let (config_ident, config) = self.generate_enum(GenerationTarget::Config);
        let deserialize_fns = self.generate_deserialize_fns();
        let into_args_fn = self.generate_into_args_fn();
        let from_args_fn = self.generate_from_args_fn();

        quote! {
            #[derive(::clap::Subcommand)]
            #opts

            #config

            impl #config_ident {
                #deserialize_fns
            }

            impl ::clap_config_fallback::IntoArgs for #opts_ident {
                #into_args_fn
            }

            impl ::clap_config_fallback::FromArgs for #opts_ident {
                #from_args_fn
            }

            impl ::clap_config_fallback::ConfigFallback for #ident {
                type Opts = #opts_ident;
                type Config = #config_ident;
            }
        }
    }
}

impl<T: EnumLike> EnumGenerator<T> {
    /// Creates a new `EnumGenerator` for the given derive input.
    pub fn new(input: T) -> Self {
        Self { input }
    }

    fn generate_enum(&self, target: GenerationTarget) -> (Ident, TokenStream) {
        let ident = format_ident!("{}{}", self.input.ident(), target.suffix());
        let variants = self
            .input
            .variants()
            .iter()
            .filter(|variant| !target.should_skip(*variant))
            .map(|variant| {
                let variant_ident = &variant.ident();

                match variant.shape() {
                    VariantShape::Unit => quote! { #variant_ident },
                    VariantShape::Newtype(ty) => {
                        let target_ident = target.suffix_ident();

                        quote! { #variant_ident(<#ty as ::clap_config_fallback::ConfigFallback>::#target_ident) }
                    }
                    VariantShape::Struct(fields) => {
                        let ident = format_ident!("{}{}", self.input.ident(), target.suffix());
                        let fields = fields
                            .iter()
                            .filter(|field| !target.should_skip(*field))
                            .map(|field| {
                                helpers::generate_field_definition(
                                    &ident,
                                    field,
                                    Some(variant.ident()),
                                    target,
                                )
                            });

                        quote! { #variant_ident { #(#fields),* } }
                    }
                }
            });

        let tag_attr = self.input.tag().map(|tag| quote! { #[serde(tag = #tag)] });

        (
            ident.clone(),
            quote! {
                #[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
                #[serde(rename_all = "snake_case")]
                #tag_attr
                pub enum #ident {
                    #(#variants,)*
                }
            },
        )
    }

    fn generate_into_args_fn(&self) -> TokenStream {
        let ident = format_ident!("__args");
        let variant_matches = self.input.variants().iter().map(|variant| {
            let variant_ident = variant.ident();
            let formatted_variant = variant_ident.to_string().to_kebab_case();

            match variant.shape() {
                VariantShape::Unit => quote! {
                    Self::#variant_ident => {
                        #ident.push(#formatted_variant.to_string());
                    }
                },
                VariantShape::Newtype(_) => quote! {
                    Self::#variant_ident(value) => {
                        #ident.push(#formatted_variant.to_string());
                        #ident.extend(value.into_args());
                    }
                },
                VariantShape::Struct(fields) => {
                    let (field_idents, field_statements): (Vec<_>, Vec<_>) = fields
                        .iter()
                        .map(|field| {
                            (
                                field.ident(),
                                helpers::generate_into_args_statement(&ident, field),
                            )
                        })
                        .unzip();

                    quote! {
                        Self::#variant_ident { #(#field_idents,)* } => {
                            #ident.push(#formatted_variant.to_string());

                            #(#field_statements)*
                        }
                    }
                }
            }
        });

        quote! {
            fn into_args(self) -> impl ::std::iter::Iterator<Item = ::std::string::String> {
                let mut #ident = Vec::new();

                match self {
                    #(#variant_matches,)*
                }

                #ident.into_iter()
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

                    quote! {
                        ::std::option::Option::Some((#formatted_variant, args)) =>
                            <#ty as ::clap_config_fallback::ConfigFallback>::#target_ident::from_args(args).map(Self::#ident)
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
