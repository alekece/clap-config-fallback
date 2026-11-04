use darling::{Error, FromDeriveInput, ast::Data, util::Ignored};
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
/// - propagating global flags like `skip_all` to each variant.
///
/// The CLI alone selects the subcommand: the configuration holds one optional section per
/// variant and only the selected one is used.
///
/// # Attributes
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
