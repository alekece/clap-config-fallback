use darling::{Error, FromDeriveInput};
use derive_more::Deref;
use syn::Ident;

use crate::{
    derive::{ConfigParser, NamedField},
    generator::StructLike,
};

/// Wrapper aound [`ConfigParser`] for `Args`-specific parsing and code generation.
#[derive(Deref)]
pub struct ConfigArgs(ConfigParser);

impl FromDeriveInput for ConfigArgs {
    fn from_derive_input(input: &syn::DeriveInput) -> Result<Self, Error> {
        ConfigParser::from_derive_input(input).map(Self)
    }
}

impl StructLike for ConfigArgs {
    fn ident(&self) -> &Ident {
        self.0.ident()
    }

    fn fields(&self) -> &[NamedField] {
        self.0.fields()
    }
}
