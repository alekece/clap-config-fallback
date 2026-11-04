use darling::{
    Error, FromDeriveInput,
    ast::{Data, Fields},
    util::Ignored,
};
use syn::{Attribute, Ident};

use crate::{
    derive::{ClapCommandAttr, ConfigPrecedence, NamedField, RenameRule},
    generator::StructLike,
};

/// Parser for the `ConfigParser` derive macro, extracting struct and field information along with
/// custom attributes.
#[derive(FromDeriveInput)]
#[darling(
    attributes(config),
    forward_attrs(command),
    supports(struct_named),
    and_then = Self::autocorrect
)]
pub struct ConfigParser {
    /// The identifier of the struct being derived.
    ident: Ident,
    /// Forwarded `#[command(...)]` attributes of the struct.
    attrs: Vec<Attribute>,
    /// The data of the struct, containing its fields and their attributes.
    data: Data<Ignored, NamedField>,
    /// Whether to skip all fields, making the configutation struct effectively empty.
    #[darling(default)]
    skip_all: bool,
    #[darling(default)]
    precedence: Option<ConfigPrecedence>,
    #[darling(skip)]
    command: ClapCommandAttr,
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

    fn attributes(&self) -> &[Attribute] {
        &self.attrs
    }

    fn rename_rule(&self) -> RenameRule {
        self.command.rename_all().unwrap_or_default()
    }
}

impl ConfigParser {
    /// Parses the naming items of `#[command(...)]` and propagates the `skip_all` flag to all fields, marking them as skipped if `skip_all` is enabled.
    fn autocorrect(mut self) -> Result<Self, Error> {
        self.command = ClapCommandAttr::from_attrs(&self.attrs)?;

        match &mut self.data {
            Data::Struct(Fields { fields, .. }) => fields.iter_mut().for_each(|field| {
                field.skip |= self.skip_all;

                if field.precedence.is_none() {
                    field.precedence = self.precedence;
                }
            }),
            _ => unreachable!(),
        }

        Ok(self)
    }
}
