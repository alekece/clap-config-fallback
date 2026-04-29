use darling::{ast::Data, util::Ignored, Error, FromDeriveInput};
use syn::Ident;

use crate::{derive::Variant, generator::EnumLike};

/// Parser for the `ConfigSubcommand` derive macro.
///
/// This structure extracts metadata from an enum representing CLI subcommands,
/// along with additional configuration-specific attributes.
///
/// It is responsible for:
/// - capturing the enum identifier,
/// - collecting its variants,
/// - handling configuration-related attributes such as `tag`,
/// - propagating global flags like `skip_all` to each variant.
///
/// # Attributes
///
/// ## `#[config(tag = "...")]`
///
/// Defines the name of the field used in the configuration file to identify
/// which subcommand variant should be selected.
///
/// For example:
///
/// ```toml
/// [command]
/// ref = "debug"
/// ```
///
/// with:
///
/// ```rust
/// #[derive(ConfigSubcommand)]
/// #[config(tag = "ref")]
/// enum Command { ... }
/// ```
///
/// ## `#[config(skip_all)]`
///
/// When enabled, all variants will be marked as skipped during configuration
/// generation, effectively disabling config parsing for this enum.
#[derive(FromDeriveInput)]
#[darling(attributes(config), supports(enum_any), and_then = Self::autocorrect)]
pub struct ConfigSubcommand {
    /// The identifier of the enum on which the derive macro is applied.
    ident: Ident,
    /// The data of the enum, containing its variants and their attributes.
    data: Data<Variant, Ignored>,
    /// If `true`, all variants will be marked as skipped during generation.
    #[darling(default)]
    skip_all: bool,
    /// Name of the configuration field used to discriminate enum variants.
    ///
    /// This corresponds to the **tag** in an internally tagged enum representation.
    /// It must be explicitly provided via `#[config(tag = "...")]`.
    tag: String,
}

impl EnumLike for ConfigSubcommand {
    fn ident(&self) -> &Ident {
        &self.ident
    }

    fn variants(&self) -> &[Variant] {
        match &self.data {
            Data::Enum(variants) => variants,
            Data::Struct(_) => unreachable!(),
        }
    }
}

impl ConfigSubcommand {
    /// Returns the configured tag field name.
    pub fn tag(&self) -> &str {
        self.tag.as_str()
    }

    /// Applies post-processing the the parsed enum.
    ///
    /// If `skip_all` is enabled, this will mark all variants as skipped.
    pub fn autocorrect(mut self) -> Result<Self, Error> {
        match &mut self.data {
            Data::Enum(variants) => variants
                .iter_mut()
                .for_each(|variant| variant.skip |= self.skip_all),
            _ => unreachable!(),
        }

        Ok(self)
    }
}
