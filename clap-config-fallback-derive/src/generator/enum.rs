use heck::{ToKebabCase, ToSnakeCase};
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
        let (config_ident, config) = self.generate_config_struct();
        let merge_command_impl = self.generate_merge_command_impl(&opts_ident, &config_ident);
        let extend_arg_fn = self.generate_extend_args_fn();
        let from_args_fn = self.generate_from_args_fn();

        quote! {
            #[derive(::clap::Subcommand)]
            #opts

            #config

            #merge_command_impl

            impl ::clap_config_fallback::IntoArgs for #opts_ident {
                #extend_arg_fn
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

    fn non_skipped_config_variants(&self) -> impl Iterator<Item = &Variant> {
        self.input
            .variants()
            .iter()
            .filter(|v| !GenerationTarget::Config.should_skip(*v))
    }

    /// Generates the struct-based `*Config` type and any inline per-variant structs.
    fn generate_config_struct(&self) -> (Ident, TokenStream) {
        let enum_ident = self.input.ident();
        let config_ident = format_ident!("{}Config", enum_ident);

        let mut inline_structs = TokenStream::new();
        let mut fields = Vec::new();

        for variant in self.non_skipped_config_variants() {
            let variant_ident = variant.ident();
            let field_name = format_ident!("{}", variant_ident.to_string().to_snake_case());

            match variant.shape() {
                VariantShape::Unit => {
                    // A bool flag: `run = true` selects this unit variant.
                    fields.push(quote! {
                        #[serde(default)]
                        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                        pub #field_name: ::std::option::Option<bool>
                    });
                }
                VariantShape::Newtype(ty) => {
                    fields.push(quote! {
                        #[serde(default)]
                        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                        pub #field_name: ::std::option::Option<<#ty as ::clap_config_fallback::ConfigFallback>::Config>
                    });
                }
                VariantShape::Struct(variant_fields) => {
                    let inline_ident = format_ident!("{}{}", enum_ident, variant_ident);
                    let struct_fields = variant_fields
                        .iter()
                        .filter(|f| !GenerationTarget::Config.should_skip(*f))
                        .map(|f| {
                            helpers::generate_field_definition(
                                &inline_ident,
                                f,
                                None,
                                GenerationTarget::Config,
                            )
                        });
                    let deserialize_fns: TokenStream = variant_fields
                        .iter()
                        .filter(|f| !GenerationTarget::Config.should_skip(*f))
                        .map(|f| helpers::generate_deserialize_fn(f, None))
                        .into_token_stream();

                    inline_structs.extend(quote! {
                        #[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]
                        #[serde(default)]
                        pub struct #inline_ident {
                            #(#struct_fields),*
                        }

                        impl #inline_ident {
                            #deserialize_fns
                        }
                    });

                    fields.push(quote! {
                        #[serde(default)]
                        #[serde(skip_serializing_if = "::std::option::Option::is_none")]
                        pub #field_name: ::std::option::Option<#inline_ident>
                    });
                }
            }
        }

        fields.push(quote! {
            #[serde(default)]
            #[serde(skip_serializing_if = "::std::option::Option::is_none")]
            pub fallback_command: ::std::option::Option<::std::string::String>
        });

        let config_struct = quote! {
            #[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]
            #[serde(default, deny_unknown_fields)]
            pub struct #config_ident {
                #(#fields),*
            }

            #inline_structs
        };

        (config_ident, config_struct)
    }

    /// Generates the `MergeCommand` impl for the struct-based `*Config` type.
    fn generate_merge_command_impl(&self, opts_ident: &Ident, config_ident: &Ident) -> TokenStream {
        let non_skipped: Vec<_> = self.non_skipped_config_variants().collect();

        // Match arms for when the CLI selected a command.
        let cli_present_arms = non_skipped.iter().map(|variant| {
            let variant_ident = variant.ident();
            let field_name = format_ident!("{}", variant_ident.to_string().to_snake_case());

            match variant.shape() {
                VariantShape::Unit => quote! {
                    ::std::option::Option::Some(#opts_ident::#variant_ident) => {
                        ::std::result::Result::Ok(::std::option::Option::Some(
                            #opts_ident::#variant_ident
                        ))
                    }
                },
                VariantShape::Newtype(_) => quote! {
                    ::std::option::Option::Some(#opts_ident::#variant_ident(__cli)) => {
                        ::std::result::Result::Ok(::std::option::Option::Some(
                            #opts_ident::#variant_ident(
                                match __cfg.#field_name {
                                    ::std::option::Option::Some(__c) => __c.merge_into_opts(__cli),
                                    ::std::option::Option::None => __cli,
                                }
                            )
                        ))
                    }
                },
                VariantShape::Struct(fields) => {
                    let field_idents: Vec<_> = fields
                        .iter()
                        .filter(|f| !GenerationTarget::Config.should_skip(*f))
                        .map(|f| f.ident().clone())
                        .collect();
                    let merge_exprs = field_idents.iter().map(|fid| {
                        quote! {
                            #fid: #fid.or_else(|| __cfg.#field_name.as_ref().and_then(|__d| __d.#fid.clone()))
                        }
                    });

                    quote! {
                        ::std::option::Option::Some(#opts_ident::#variant_ident { #(#field_idents,)* }) => {
                            ::std::result::Result::Ok(::std::option::Option::Some(
                                #opts_ident::#variant_ident { #(#merge_exprs,)* }
                            ))
                        }
                    }
                }
            }
        });

        // Match arms for `fallback_command` promotion: each snake_case variant name.
        let fallback_arms: Vec<_> = non_skipped
            .iter()
            .map(|variant| {
                let variant_ident = variant.ident();
                let field_name = format_ident!("{}", variant_ident.to_string().to_snake_case());
                let name_literal = variant_ident.to_string().to_snake_case();

                match variant.shape() {
                    VariantShape::Unit => quote! {
                        ::std::option::Option::Some(#name_literal) => {
                            return ::std::result::Result::Ok(
                                ::std::option::Option::Some(#opts_ident::#variant_ident)
                            );
                        }
                    },
                    VariantShape::Newtype(_) => quote! {
                        ::std::option::Option::Some(#name_literal) => {
                            return ::std::result::Result::Ok(
                                ::std::option::Option::Some(
                                    #opts_ident::#variant_ident(
                                        __cfg.#field_name.unwrap_or_default()
                                            .merge_into_opts(::std::default::Default::default())
                                    )
                                )
                            );
                        }
                    },
                    VariantShape::Struct(fields) => {
                        let field_idents: Vec<_> = fields
                            .iter()
                            .filter(|f| !GenerationTarget::Config.should_skip(*f))
                            .map(|f| f.ident().clone())
                            .collect();

                        quote! {
                            ::std::option::Option::Some(#name_literal) => {
                                let __d = __cfg.#field_name.unwrap_or_default();
                                return ::std::result::Result::Ok(
                                    ::std::option::Option::Some(
                                        #opts_ident::#variant_ident {
                                            #(#field_idents: __d.#field_idents,)*
                                        }
                                    )
                                );
                            }
                        }
                    }
                }
            })
            .collect();

        quote! {
            impl ::clap_config_fallback::MergeCommand for #config_ident {
                type CommandOpts = #opts_ident;

                fn merge_command(
                    self,
                    cli_command: ::std::option::Option<#opts_ident>,
                ) -> ::std::result::Result<::std::option::Option<#opts_ident>, ::std::string::String> {
                    let __cfg = self;

                    match cli_command {
                        #(#cli_present_arms)*
                        // Skipped variants (not in config struct) are passed through unchanged.
                        ::std::option::Option::Some(__other) => {
                            ::std::result::Result::Ok(::std::option::Option::Some(__other))
                        }
                        ::std::option::Option::None => {
                            match __cfg.fallback_command.as_deref() {
                                #(#fallback_arms)*
                                ::std::option::Option::Some(__other) => {
                                    ::std::result::Result::Err(
                                        ::std::format!("unknown fallback command: {}", __other)
                                    )
                                }
                                ::std::option::Option::None => {
                                    ::std::result::Result::Ok(::std::option::Option::None)
                                }
                            }
                        }
                    }
                }
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

                    // Use unwrap_or_default so that selecting this subcommand on the CLI is always
                    // captured, even when none of its arguments were explicitly provided.
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
}
