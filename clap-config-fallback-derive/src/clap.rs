use darling::{Error, FromMeta, util::Override};
use heck::ToKebabCase;
use syn::{Attribute, Expr, Ident, Meta, Token, parse_quote, punctuated::Punctuated};

use crate::syn_utils::ExprExt;

#[derive(FromMeta)]
pub enum ClapCommand {
    Flatten,
    Subcommand,
}

impl ClapCommand {
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        Self::from_meta(&attr.meta).ok()
    }
}

/// Represents a shorthand for `clap` argument attributes.
#[derive(Debug, FromMeta)]
pub struct ClapArg {
    /// Optional override for the short flag (e.g., `-d`).
    short: Option<Override<char>>,
    /// Optional override for the long flag (e.g., `--debug`).
    long: Option<Override<String>>,
    /// Optional default value for the argument.
    default_value: Option<String>,
    /// Catch-all fields used to absorb ignored clap attributes.
    /// This prevents darling from failing when encountering unsupported args.
    #[allow(dead_code)]
    #[darling(flatten)]
    ignored_args: Result<(), Error>,
}

impl ClapArg {
    /// Creates a `ClapArgAttribute` from an `Attribute`, extracting relevant information from its
    /// meta.
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        Self::from_meta(&attr.meta).ok()
    }

    /// Returns the default value for this argument, if specified.
    pub fn default_value(&self) -> Option<&str> {
        self.default_value.as_deref()
    }

    /// Generates the appropriate short or long flag for this argument based on the provided field
    ///
    /// Note that if both `short` and `long` are specified, `short` will take precedence for
    /// generating the flag
    pub fn flag_name(&self, field_ident: &Ident) -> Option<String> {
        if let Some(short) = &self.short {
            return match short {
                Override::Explicit(c) => Some(format!("-{c}")),
                Override::Inherit => field_ident
                    .to_string()
                    .chars()
                    .next()
                    .map(|c| format!("-{}", c.to_lowercase())),
            };
        }

        if let Some(long) = &self.long {
            return match long {
                Override::Explicit(s) => Some(format!("--{s}")),
                Override::Inherit => Some(format!("--{}", field_ident.to_string().to_kebab_case())),
            };
        }

        None
    }

    /// Sanitizes the given attribute by removing any sub-attributes that are not relevant for
    /// optional argument.
    pub fn sanitize(attr: Attribute) -> Attribute {
        const DENIED_ARGS: [&str; 16] = [
            "default_value",
            "default_values",
            "default_value_if",
            "default_value_ifs",
            "required",
            "required_if_eq",
            "required_if_eq_any",
            "required_unless_present",
            "required_unless_present_any",
            "required_unless_present_all",
            "requires",
            "requires_if",
            "requires_ifs",
            "conflicts_with",
            "conflicts_with_all",
            "exclusive",
        ];

        if attr.path().is_ident("arg")
            && let Meta::List(list) = &attr.meta
            && let Some(args) = list
                .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                .ok()
        {
            let sanitized_attrs: Punctuated<Expr, Token![,]> = args
                .into_iter()
                .filter(|expr| {
                    !expr
                        .ident()
                        .is_none_or(|ident| DENIED_ARGS.contains(&ident.to_string().as_str()))
                })
                .collect();

            parse_quote!(#[arg(#sanitized_attrs)])
        } else {
            attr
        }
    }
}
