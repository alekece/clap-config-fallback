use darling::{Error, FromField};
use derive_more::{Deref, DerefMut};
use syn::{Attribute, Expr, Ident, LitStr, Type};

use crate::{
    TypeExt,
    derive::{ClapArg, ClapCommand, ConfigFormat, Skippable},
};

/// Named-field wrapper used by derives that only support struct-style fields.
#[derive(Deref, DerefMut)]
pub struct NamedField(Field);

impl FromField for NamedField {
    fn from_field(field: &syn::Field) -> Result<Self, Error> {
        Field::from_field(field).and_then(Self::try_from)
    }
}

impl TryFrom<Field> for NamedField {
    type Error = Error;

    fn try_from(field: Field) -> Result<Self, Self::Error> {
        if field.ident.is_none() {
            Err(Error::custom("tuple fields are not supported").with_span(&field.ty))
        } else {
            Ok(Self(field))
        }
    }
}

impl Skippable for NamedField {
    fn is_skipped(&self) -> bool {
        self.0.is_skipped()
    }
}

impl NamedField {
    /// Returns the field identifier, always present for named field.
    pub fn ident(&self) -> &Ident {
        self.0.ident().as_ref().unwrap()
    }
}

/// Parser for individual fields, extracting field information and custom attributes related to
/// configuration parsing and command-line argument generation.
#[derive(Clone, FromField)]
#[darling(attributes(config), forward_attrs(arg, command), and_then = Self::autocorrect)]
pub struct Field {
    /// The identifier of the field.
    ident: Option<Ident>,
    /// The type of the field.
    ty: Type,
    /// The attributes applied to the field, fowarding only those relevant to `arg` and `command`
    /// for further processing.
    attrs: Vec<Attribute>,
    /// Whether to skip this field in the generated configuration struct, marked with
    /// `#[config(skip)]` or inherited from `#[config(skip_all)`].
    #[darling(default)]
    pub(crate) skip: bool,
    /// Whether this field is the configuration path field, marked with `#[config(path)]`.
    #[darling(default)]
    path: bool,
    /// The format of the configuration file, marked with `#[config(format = "toml")]` or similar.
    /// If not specified, it will be determined automatically based on the file extension of the
    /// path field, if any.
    #[darling(default)]
    format: Option<ConfigFormat>,
    /// An optional expression specifying how to format the value of this field when generating the
    /// final command-line arguments.
    /// Defaults to `ToString::to_string()` if not specified.
    #[darling(default)]
    value_format: Option<Expr>,
    #[darling(default)]
    alias: Option<LitStr>,
    #[darling(default)]
    aliases: Option<Vec<LitStr>>,
    #[darling(default)]
    no_flatten: bool,
    #[darling(skip)]
    commands: Option<Vec<ClapCommand>>,
    #[darling(skip)]
    args: Option<Vec<ClapArg>>,
}

impl Skippable for Field {
    fn is_skipped(&self) -> bool {
        // path fields should not be generated in config type
        self.skip || self.path
    }
}

impl Field {
    /// Returns the identifier of the field.
    pub fn ident(&self) -> Option<&Ident> {
        self.ident.as_ref()
    }

    /// Returns the type of the field.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    /// Returns forwarded `#[arg(...)]` and `#[command(...)]` attributes.
    pub fn attributes(&self) -> &[Attribute] {
        &self.attrs
    }

    /// Returns whether this field is marked as the configuration path field, which indicates that it
    /// should be used to determine the path of the configuration file when parsing command-line
    /// arguments.
    pub fn is_path(&self) -> bool {
        self.path
    }

    /// Returns the configuration format specified for this field, or `ConfigFormat::Auto` if not.
    pub fn format(&self) -> Option<ConfigFormat> {
        self.format
    }

    /// Returns the optional expression specifying how to format the value of this field.
    pub fn value_format(&self) -> Option<&Expr> {
        self.value_format.as_ref()
    }

    pub fn aliases(&self) -> Vec<LitStr> {
        self.aliases
            .iter()
            .flatten()
            .chain(&self.alias)
            .cloned()
            .collect()
    }

    pub fn flatten(&self) -> bool {
        !self.no_flatten && self.commands().contains(&ClapCommand::Flatten)
    }

    pub fn commands(&self) -> &[ClapCommand] {
        self.commands.as_ref().unwrap()
    }

    pub fn args(&self) -> &[ClapArg] {
        self.args.as_ref().unwrap()
    }

    /// Verify that the path field is a `String` or `Option<String>`, as required for determining
    /// the configuration file path.
    fn autocorrect(mut self) -> Result<Self, Error> {
        let mut error = Error::accumulator();

        self.commands = Some(
            self.attrs
                .iter()
                .filter_map(ClapCommand::from_attr)
                .collect(),
        );
        self.args = Some(self.attrs.iter().filter_map(ClapArg::from_attr).collect());

        if !self.aliases().is_empty() && self.commands().is_empty() {
            error.push(
                    Error::custom(r#"#[config(alias, aliases)] can only be used on fields with `#[command]` attributes"#)
                        .with_span(&self.ident)
                );
        }

        if !self.aliases().is_empty() && self.flatten() {
            error.push(
                Error::custom(r#"#[config(alias, aliases)] can only used on non-flatten fields"#)
                    .with_span(&self.ident),
            );
        }

        if self.path && !(self.ty.is("String") || self.ty.is_option_of("String")) {
            error.push(
                Error::custom(
                    "`#[config(path)]` requires a field of type `String` or `Option<String>`",
                )
                .with_span(&self.ident),
            );
        }

        error.finish().map(|_| self)
    }
}
