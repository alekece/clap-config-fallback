use darling::{Error, FromMeta};
use quote::format_ident;
use syn::{Ident, LitStr, Meta};

use crate::generator::GenerationTarget;

/// Custom names for the generated types, set with `#[config(opts = "...", config = "...")]`.
#[derive(Default, FromMeta)]
pub struct TypeNames {
    #[darling(default, with = Self::parse_name)]
    opts: Option<Ident>,
    #[darling(default, with = Self::parse_name)]
    config: Option<Ident>,
}

impl TypeNames {
    /// Returns the custom name for `target`, otherwise `<base><Suffix>`.
    pub fn ident(&self, base: &Ident, target: GenerationTarget) -> Ident {
        let custom = match target {
            GenerationTarget::Opts => &self.opts,
            GenerationTarget::Config => &self.config,
        };

        custom
            .clone()
            .unwrap_or_else(|| format_ident!("{}{}", base, target.suffix()))
    }

    fn parse_name(meta: &Meta) -> Result<Option<Ident>, Error> {
        let name = LitStr::from_meta(meta)?;

        syn::parse_str(&name.value()).map(Some).map_err(|_| {
            Error::custom(format!("`{}` is not a valid type name", name.value())).with_span(&name)
        })
    }
}
