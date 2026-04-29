use darling::{
    Error, FromVariant,
    ast::{Fields, Style},
};
use syn::{Ident, Type};

use crate::{
    TypeExt,
    derive::{Field, NamedField, Skippable},
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
#[darling(attributes(config),  and_then = Self::autocorrect)]
pub struct Variant {
    ident: Ident,
    fields: Fields<Field>,
    #[darling(default)]
    pub(crate) skip: bool,
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
