use darling::{Error, FromVariant, ast::{Fields, Style}, error::Accumulator};
use syn::{Ident, Type};

use crate::{TypeExt, derive::{Field, NamedField, Skippable}};

pub enum VariantKind {
    Unit,
    Newtype(Type),
    Struct(Vec<NamedField>),
}

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
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    pub fn kind(&self) -> VariantKind {
        if self.is_unit() {
            VariantKind::Unit
        } else if let Some(ty) = self.as_newtype() {
            VariantKind::Newtype(ty)
        } else if let Some(fields) = self.as_struct() {
            VariantKind::Struct(fields)
        } else {
            unreachable!("variant must be either empty, newtype, or struct")
        }
    }

    pub fn is_unit(&self) -> bool {
        self.fields.is_unit()
    }

    pub fn as_newtype(&self) -> Option<Type> {
        self.fields
            .is_newtype()
            .then(|| self.fields.fields.first().unwrap().ty().clone())
    }

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

    fn autocorrect(mut self) -> Result<Self, Error> {
        let mut accumulator = Accumulator::default();

        if let Some(ty) = self.as_newtype() && ty.is_unit() {
            self.fields = Fields::new(Style::Unit, Vec::default());
        }

        if self.fields.is_tuple() && self.fields.fields.len() != 1 {
            accumulator.push(
                Error::custom("tuple enums are not supported")
                    .with_span(self.ident())
            );
        }

        if self.fields.is_struct() {
            for field in self.fields.fields.iter() {
                if field.ident().is_none() {
                    accumulator.push(
                        Error::custom("struct enums requires named fields").with_span(field.ty()),
                    );
                }
            }
        }

        accumulator.finish().map(|_| self)
    }
}
