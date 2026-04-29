use darling::{
    Error, FromDeriveInput,
    ast::{Data, Fields},
    util::Ignored,
};
use syn::Ident;

use crate::{derive::NamedField, generator::StructLike};

/// Parser for the `ConfigParser` derive macro, extracting struct and field information along with
/// custom attributes.
#[derive(FromDeriveInput)]
#[darling(attributes(config), supports(struct_named), and_then = Self::autocorrect)]
pub struct ConfigParser {
    /// The identifier of the struct being derived.
    ident: Ident,
    /// The data of the struct, containing its fields and their attributes.
    data: Data<Ignored, NamedField>,
    /// Whether to skip all fields, making the configutation struct effectively empty.
    #[darling(default)]
    skip_all: bool,
}

impl StructLike for ConfigParser {
    fn ident(&self) -> &Ident {
        &self.ident
    }

    fn fields(&self) -> &[NamedField] {
        match &self.data {
            Data::Struct(fields) => &fields.fields,
            Data::Enum(_) => unreachable!(),
        }
    }
}

impl ConfigParser {
    /// Propagate the `skip_all` flag to all fields, marking them as skipped if `skip_all` is enabled.
    fn autocorrect(mut self) -> Result<Self, Error> {
        match &mut self.data {
            Data::Struct(Fields { fields, .. }) => fields
                .iter_mut()
                .for_each(|field| field.skip |= self.skip_all),
            _ => unreachable!(),
        }

        Ok(self)
    }
}
