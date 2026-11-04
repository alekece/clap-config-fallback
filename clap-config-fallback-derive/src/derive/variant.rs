use darling::{
    Error, FromVariant,
    ast::{Fields, Style},
};
use syn::{Attribute, Ident, Type};

use crate::{
    TypeExt,
    derive::{ClapCommandAttr, Field, NamedField, RenameRule, Skippable},
};

/// Normalized variant shape.
pub enum VariantShape {
    /// Variant without payload.
    Unit,
    /// Variant with a single field payload.
    Newtype(Type),
    /// Variant with named fields payload.
    Struct(Vec<NamedField>),
}

/// Parsed enum variant metadata.
#[derive(FromVariant)]
#[darling(attributes(config), forward_attrs(command), and_then = Self::autocorrect)]
pub struct Variant {
    ident: Ident,
    /// Forwarded `#[command(...)]` attributes of the variant.
    attrs: Vec<Attribute>,
    fields: Fields<Field>,
    #[darling(default)]
    pub(crate) skip: bool,
    #[darling(skip)]
    command: ClapCommandAttr,
}

impl Skippable for Variant {
    fn is_skipped(&self) -> bool {
        self.skip
    }
}

impl Variant {
    /// Variant identifier.
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    /// Returns forwarded `#[command(...)]` attributes.
    pub fn attributes(&self) -> &[Attribute] {
        &self.attrs
    }

    /// Returns the subcommand name clap uses: the explicit `name`, otherwise the identifier cased
    /// by the variant's own `rename_all`, falling back to the enum's.
    pub fn command_name(&self, enum_rule: RenameRule) -> String {
        self.command.name().map_or_else(
            || self.rename_rule(enum_rule).apply(&self.ident.to_string()),
            str::to_owned,
        )
    }

    /// Returns the casing of the variant's fields: its own `rename_all`, otherwise the enum's.
    pub fn rename_rule(&self, enum_rule: RenameRule) -> RenameRule {
        self.command.rename_all().unwrap_or(enum_rule)
    }

    /// Returns the normalized variant shape.
    pub fn shape(&self) -> VariantShape {
        if self.is_unit() {
            VariantShape::Unit
        } else if let Some(ty) = self.as_newtype() {
            VariantShape::Newtype(ty)
        } else if let Some(fields) = self.as_struct() {
            VariantShape::Struct(fields)
        } else {
            unreachable!("variant must be either empty, newtype, or struct")
        }
    }

    /// Returns true if the variant has no fields.
    pub fn is_unit(&self) -> bool {
        self.fields.is_unit()
    }

    /// Returns the inner field type for newtype variants.
    pub fn as_newtype(&self) -> Option<Type> {
        self.fields
            .is_newtype()
            .then(|| self.fields.fields.first().unwrap().ty().clone())
    }

    /// Returns named fields for struct variants.
    pub fn as_struct(&self) -> Option<Vec<NamedField>> {
        self.fields.is_struct().then(|| {
            self.fields
                .fields
                .iter()
                .cloned()
                .map(|field| field.try_into().unwrap())
                .collect()
        })
    }

    /// Normalizes the variant shape and applies consistency checks.
    ///
    /// Returns an error if the variant is invalid, such as tuple variants with more than one field
    /// or struct variants with unnamed fields.
    fn autocorrect(mut self) -> Result<Self, Error> {
        let mut error = Error::accumulator();

        if let Some(command) = error.handle(ClapCommandAttr::from_attrs(&self.attrs)) {
            self.command = command;
        }

        if let Some(ty) = self.as_newtype()
            && ty.is_unit()
        {
            self.fields = Fields::new(Style::Unit, Vec::default());
        }

        if self.fields.is_tuple() && self.fields.fields.len() != 1 {
            error.push(Error::custom("tuple enums are not supported").with_span(self.ident()));
        }

        if self.fields.is_struct() {
            for field in self.fields.fields.iter() {
                if field.ident().is_none() {
                    error.push(
                        Error::custom("struct enums requires named fields").with_span(field.ty()),
                    );
                }
            }
        }

        error.finish().map(|_| self)
    }
}
