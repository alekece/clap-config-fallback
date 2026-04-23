use darling::{
    Error, FromDeriveInput, FromField, FromMeta,
    ast::{Data, Fields},
    util::Ignored,
};
use syn::{Attribute, Ident, Type};

use crate::TypeExt;

/// Parser for the `ConfigParser` derive macro, extracting struct and field information along with
/// custom attributes.
#[derive(FromDeriveInput)]
#[darling(attributes(config), supports(struct_named), and_then = ConfigParser::autocorrect)]
pub struct ConfigParser {
    /// The identifier of the struct being derived.
    ident: Ident,
    /// The data of the struct, containing its fields and their attributes.
    data: Data<Ignored, ConfigParserField>,
    /// Whether to skip all fields, making the configutation struct effectively empty.
    #[darling(default)]
    skip_all: bool,
}

impl ConfigParser {
    pub fn autocorrect(mut self) -> Result<Self, Error> {
        let Data::Struct(Fields { fields, .. }) = &mut self.data else {
            return Err(
                Error::custom("`ConfigParser` can only be used with structs")
                    .with_span(&self.ident),
            );
        };

        let mut path_field = None;

        for field in fields.iter_mut() {
            field.skip = field.skip || self.skip_all || field.path;

            if field.ident.is_none() {
                return Err(Error::custom("Tuple fields are not supported").with_span(&field.ty));
            }

            if field.path {
                if path_field.is_some() {
                    return Err(Error::custom(
                        "`#[config(path)]` can only be applied to one field",
                    )
                    .with_span(&field.ident));
                }

                if !(field.ty.is("String") || field.ty.is_option_of("String")) {
                    return Err(Error::custom(
                        "`#[config(path)]` requires a field of type `String` or `Option<String>`",
                    )
                    .with_span(&field.ident));
                }

                path_field = Some(field);
            }
        }

        Ok(self)
    }

    /// Returns the identifier of the struct being derived.
    pub fn ident(&self) -> &Ident {
        &self.ident
    }

    /// Returns an iterator over the fields of the struct.
    pub fn fields(&self) -> impl Iterator<Item = &ConfigParserField> {
        match &self.data {
            Data::Struct(Fields { fields, .. }) => fields.iter(),
            _ => unreachable!(),
        }
    }
}

/// Parser for individual fields in the `ConfigParser` derive macro, extracting field information
/// and custom attributes related to configuration parsing and command-line argument generation.
#[derive(FromField)]
#[darling(attributes(config), forward_attrs(arg, command))]
pub struct ConfigParserField {
    /// The identifier of the field.
    ident: Option<Ident>,
    /// The type of the field.
    ty: Type,
    /// The attributes applied to the field, fowarding only those relevant to `arg` and `command`
    /// for further processing.
    attrs: Vec<Attribute>,
    /// Whether to skip this field in the generated configuration struct, marked with
    /// `#[config(skip)]` or inherited from `skip_all`.
    #[darling(default)]
    skip: bool,
    /// Whether this field is the configuration path field, marked with `#[config(path)]`.
    #[darling(default)]
    path: bool,
    /// The format of the configuration file, marked with `#[config(format = "toml")]` or similar.
    /// If not specified, it will be determined automatically based on the file extension of the
    /// path field, if any.
    #[darling(default)]
    format: Option<ConfigFormat>,
}

impl ConfigParserField {
    /// Returns the identifier of the field.
    pub fn ident(&self) -> &Ident {
        self.ident.as_ref().unwrap()
    }

    /// Returns the type of the field.
    pub fn ty(&self) -> &Type {
        &self.ty
    }

    pub fn attributes(&self) -> impl Iterator<Item = &Attribute> {
        self.attrs.iter()
    }

    /// Returns whether this field is marked as the configuration path field, which indicates that it
    /// should be used to determine the path of the configuration file when parsing command-line
    /// arguments.
    pub fn is_path(&self) -> bool {
        self.path
    }

    /// Returns whether this field is marked to be skipped in the generated configuration struct.
    pub fn is_skipped(&self) -> bool {
        self.skip
    }

    /// Returns the configuration format specified for this field, or `ConfigFormat::Auto` if not.
    pub fn format(&self) -> Option<ConfigFormat> {
        self.format
    }
}

/// `format` attribute for configuration files, allowing explicit specification of the format or
/// automatic detection based on file extension.
#[derive(Copy, Clone, Default, FromMeta)]
pub enum ConfigFormat {
    /// Explicitly specify that the configuration file is in TOML format.
    Toml,
    /// Explicitly specify that the configuration file is in YAML format.
    Yaml,
    /// Explicitly specify that the configuration file is in JSON format.
    Json,
    /// Indicates that the configuration format should be determined automatically based on the file
    /// extension of the path field, if any.
    #[default]
    Auto,
}
