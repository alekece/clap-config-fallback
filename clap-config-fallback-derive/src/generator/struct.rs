use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::{
    ConfigArgs, TypeExt,
    derive::{ClapCommand, ConfigFormat, ConfigParser, NamedField},
    generator::{GenerationTarget, helpers},
    syn_utils::IntoTokenStream,
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
        let extend_args_fn = self.generate_extend_args_fn();
        let from_args_fn = self.generate_from_args_fn();
        let deserialize_fns = self.generate_deserialize_fns();
        let config_path_fn = self.generate_config_path_fn();
        let config_format_fn = self.generate_config_format_fn();
        let command_aware_impl =
            self.generate_command_aware_impl(ident, &opts_ident, &config_ident);

        quote! {
            #config

            impl #config_ident {
                #deserialize_fns
            }

            #[derive(::clap::Parser)]
            #opts

            impl ::clap_config_fallback::IntoArgs for #opts_ident {
                #extend_args_fn
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

            #command_aware_impl

            impl ::clap_config_fallback::ConfigParser for #ident {

            }
        }
    }

    fn generate_command_aware_impl(
        &self,
        ident: &Ident,
        opts_ident: &Ident,
        config_ident: &Ident,
    ) -> TokenStream {
        let command_field = self
            .input
            .fields()
            .iter()
            .find(|f| f.commands().contains(&ClapCommand::Subcommand));

        if let Some(field) = command_field {
            let field_ident = field.ident();
            let field_ty = field.ty().clone().unwrap_option();

            quote! {
                impl ::clap_config_fallback::CommandAware for #ident {
                    type CommandOpts = <#field_ty as ::clap_config_fallback::ConfigFallback>::Opts;
                    type CommandConfig = <#field_ty as ::clap_config_fallback::ConfigFallback>::Config;

                    fn take_opts_command(
                        opts: &mut #opts_ident,
                    ) -> ::std::option::Option<Self::CommandOpts> {
                        opts.#field_ident.take()
                    }

                    fn take_config_command(
                        config: &mut #config_ident,
                    ) -> ::std::option::Option<Self::CommandConfig> {
                        config.#field_ident.take()
                    }

                    fn put_opts_command(
                        opts: &mut #opts_ident,
                        cmd: ::std::option::Option<Self::CommandOpts>,
                    ) {
                        opts.#field_ident = cmd;
                    }
                }
            }
        } else {
            quote! {
                impl ::clap_config_fallback::CommandAware for #ident {
                    type CommandOpts = ();
                    type CommandConfig = ();

                    fn take_opts_command(_: &mut #opts_ident) -> ::std::option::Option<()> {
                        ::std::option::Option::None
                    }

                    fn take_config_command(_: &mut #config_ident) -> ::std::option::Option<()> {
                        ::std::option::Option::None
                    }

                    fn put_opts_command(_: &mut #opts_ident, _: ::std::option::Option<()>) {}
                }
            }
        }
    }
}

impl StructGenerator<ConfigArgs> {
    pub fn generate(&self) -> TokenStream {
        let ident = self.input.ident();
        let (opts_ident, opts) = self.generate_struct(GenerationTarget::Opts);
        let (config_ident, config) = self.generate_struct(GenerationTarget::Config);
        let extend_args_fn = self.generate_extend_args_fn();
        let from_args_fn = self.generate_from_args_fn();
        let deserialize_fns = self.generate_deserialize_fns();
        let merge_fns = self.generate_merge_fns(&opts_ident);

        quote! {
            #config

            impl #config_ident {
                #deserialize_fns
                #merge_fns
            }

            #[derive(::clap::Args)]
            #opts

            impl ::clap_config_fallback::IntoArgs for #opts_ident {
                #extend_args_fn
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

    /// Generates `merge_into_opts` on the `*Config` type.
    fn generate_merge_fns(&self, opts_ident: &Ident) -> TokenStream {
        // Only non-command, non-skipped fields participate in field-level merging.
        let merge_fields: Vec<_> = self
            .input
            .fields()
            .iter()
            .filter(|f| f.commands().is_empty() && !GenerationTarget::Config.should_skip(*f))
            .collect();

        // Command fields in Args structs are always taken from the CLI value unchanged.
        let command_fields: Vec<_> = self
            .input
            .fields()
            .iter()
            .filter(|f| !f.commands().is_empty())
            .collect();

        let merge_field_idents: Vec<_> = merge_fields.iter().map(|f| f.ident()).collect();
        let command_field_idents: Vec<_> = command_fields.iter().map(|f| f.ident()).collect();

        let merge_assignments = merge_field_idents.iter().map(|id| {
            quote! { #id: __cli.#id.or(self.#id) }
        });

        quote! {
            #[doc(hidden)]
            pub fn merge_into_opts(self, __cli: #opts_ident) -> #opts_ident {
                #opts_ident {
                    #(#merge_assignments,)*
                    #(#command_field_idents: __cli.#command_field_idents,)*
                }
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
        let ident = format_ident!("__clap_self");
        let (field_assignments, field_checks) = self
            .input
            .fields()
            .iter()
            .map(|field| {
                let field_ident = field.ident();

                (
                    helpers::generate_from_args_initializer(field),
                    quote! { #ident.#field_ident.is_none() },
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
                let #ident = Self {
                    #(#field_assignments,)*
                };

                if #field_checks {
                    None
                } else {
                    Some(#ident)
                }
            }
        }
    }

    fn generate_extend_args_fn(&self) -> TokenStream {
        let ident = format_ident!("__clap_arg");
        let field_idents = self.input.fields().iter().map(|field| field.ident());
        let field_args = self
            .input
            .fields()
            .iter()
            .map(|field| helpers::generate_extend_args_statement(&ident, field));

        quote! {
            fn extend_args(self, #ident: &mut ::std::vec::Vec<::std::ffi::OsString>) {
                #(let #field_idents = self.#field_idents;)*

                #(#field_args)*
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
            .map(|field_ident| quote! { #ident.#field_ident.as_ref().map(::std::path::Path::new) })
            .unwrap_or_else(|| quote! { ::std::option::Option::None });

        quote! {
            fn config_path(&self) -> ::std::option::Option<&::std::path::Path> {
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
        self.input
            .fields()
            .iter()
            .filter(|field| !GenerationTarget::Config.should_skip(*field))
            .map(|field| helpers::generate_deserialize_fn(field, None))
            .into_token_stream()
    }
}
