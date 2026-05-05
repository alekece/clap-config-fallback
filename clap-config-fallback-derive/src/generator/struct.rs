use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{
    ConfigArgs,
    derive::{ConfigFormat, ConfigParser, NamedField},
    generator::{GenerationTarget, helpers},
};

/// Common interface for parsed derive input that behave like structs.
pub trait StructLike {
    /// Identifier of the struct.
    fn ident(&self) -> &Ident;
    /// Fields of the struct.
    fn fields(&self) -> &[NamedField];
}

/// Generatates `Opts` and `Config` helper structs and impls for derive inputs.
pub struct StructGenerator<T: StructLike> {
    input: T,
}

impl StructGenerator<ConfigParser> {
    pub fn generate(&self) -> TokenStream {
        let ident = self.input.ident();
        let (opts_ident, opts) = self.generate_struct(GenerationTarget::Opts);
        let (config_ident, config) = self.generate_struct(GenerationTarget::Config);
        let into_args_fn = self.generate_into_args_fn();
        let from_args_fn = self.generate_from_args_fn();
        let deserialize_fns = self.generate_deserialize_fns();
        let config_path_fn = self.generate_config_path_fn();
        let config_format_fn = self.generate_config_format_fn();

        quote! {
            #config

            impl #config_ident {
                #deserialize_fns
            }

            #[derive(::clap::Parser)]
            #opts

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

            impl ::clap_config_fallback::ConfigFallback for #ident {
                type Opts = #opts_ident;
                type Config = #config_ident;
            }

            impl ::clap_config_fallback::ConfigParser for #ident {

            }
        }
    }
}

impl StructGenerator<ConfigArgs> {
    pub fn generate(&self) -> TokenStream {
        let ident = self.input.ident();
        let (opts_ident, opts) = self.generate_struct(GenerationTarget::Opts);
        let (config_ident, config) = self.generate_struct(GenerationTarget::Config);
        let into_args_fn = self.generate_into_args_fn();
        let from_args_fn = self.generate_from_args_fn();
        let deserialize_fns = self.generate_deserialize_fns();

        quote! {
            #config

            impl #config_ident {
                #deserialize_fns
            }

            #[derive(::clap::Args)]
            #opts

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

impl<T: StructLike> StructGenerator<T> {
    /// Creates a new `StructGenerator` for the given derive input.
    pub fn new(input: T) -> Self {
        Self { input }
    }

    fn generate_struct(&self, target: GenerationTarget) -> (Ident, TokenStream) {
        let ident = format_ident!("{}{}", self.input.ident(), target.suffix());
        let fields = self
            .input
            .fields()
            .iter()
            .filter(|field| !target.should_skip(*field))
            .map(|field| helpers::generate_field_definition(&ident, field, None, target));

        (
            ident.clone(),
            quote! {
                #[derive(Debug, Default, ::serde::Serialize, ::serde::Deserialize)]
                pub struct #ident {
                    #(#fields),*
                }
            },
        )
    }

    fn generate_from_args_fn(&self) -> TokenStream {
        let (field_assignments, field_checks) = self
            .input
            .fields()
            .iter()
            .map(|field| {
                let field_ident = field.ident();

                (
                    helpers::generate_from_args_initializer(field),
                    quote! { __self.#field_ident.is_none() },
                )
            })
            .unzip::<_, _, Vec<_>, Vec<_>>();

        let field_checks = if field_checks.is_empty() {
            quote! { true }
        } else {
            quote! { #(#field_checks)&&* }
        };

        quote! {
            fn from_args(args: &::clap::ArgMatches) -> Option<Self> {
                let __self = Self {
                    #(#field_assignments,)*
                };

                if #field_checks {
                    None
                } else {
                    Some(__self)
                }
            }
        }
    }

    fn generate_into_args_fn(&self) -> TokenStream {
        let ident = format_ident!("__args");
        let field_idents = self.input.fields().iter().map(|field| field.ident());
        let field_args = self
            .input
            .fields()
            .iter()
            .map(|field| helpers::generate_into_args_statement(&ident, field));

        quote! {
            fn into_args(self) -> impl ::std::iter::Iterator<Item = ::std::string::String> {
                let mut #ident = Vec::new();
                #(let #field_idents = self.#field_idents;)*

                #(#field_args)*

                #ident.into_iter()
            }
        }
    }

    fn generate_config_path_fn(&self) -> TokenStream {
        let ident = format_ident!("self");
        let config_path = self
            .input
            .fields()
            .iter()
            .find_map(|field| field.is_path().then_some(field.ident()))
            .map(|field_ident| quote! { #ident.#field_ident.as_deref() })
            .unwrap_or_else(|| quote! { ::std::option::Option::None });

        quote! {
            fn config_path(&self) -> ::std::option::Option<&str> {
                #config_path
            }
        }
    }

    fn generate_config_format_fn(&self) -> Option<TokenStream> {
        let config_format = match self
            .input
            .fields()
            .iter()
            .find(|field| field.is_path())
            .and_then(|field| field.format())
        {
            Some(ConfigFormat::Toml) => format_ident!("Toml"),
            Some(ConfigFormat::Yaml) => format_ident!("Yaml"),
            Some(ConfigFormat::Json) => format_ident!("Json"),
            // do not override the default `config_format` implementation if no format is specified or
            // if the format is set to `ConfigFormat::Auto`.
            None | Some(ConfigFormat::Auto) => return None,
        };

        Some(quote! {
            fn config_format(&self) -> ::std::option::Option<::clap_config_fallback::ConfigFormat> {
                ::std::option::Option::Some(::clap_config_fallback::ConfigFormat::#config_format)
            }
        })
    }

    fn generate_deserialize_fns(&self) -> TokenStream {
        let deserialize_fns = self
            .input
            .fields()
            .iter()
            .filter(|field| !GenerationTarget::Config.should_skip(*field))
            .map(|field| helpers::generate_deserialize_fn(field, None));

        quote! {
            #(#deserialize_fns)*
        }
    }
}
