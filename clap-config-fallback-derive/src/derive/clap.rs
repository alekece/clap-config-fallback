use darling::{Error, FromMeta, util::Override};
use heck::ToKebabCase;
use syn::{
    Attribute, Expr, ExprPath, Ident, LitStr, Meta, Token, parse_quote, punctuated::Punctuated,
};

use crate::syn_utils::ExprExt;

#[derive(Copy, Clone, PartialEq, Eq, FromMeta)]
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
#[derive(Clone, FromMeta)]
pub struct ClapArg {
    /// Optional override for the short flag (e.g., `-d`).
    short: Option<Override<char>>,
    /// Optional override for the long flag (e.g., `--debug`).
    long: Option<Override<String>>,
    /// Optional alias for the argument, allowing it to be referenced by multiple names.
    alias: Option<LitStr>,
    /// Optional aliases for the argument, allowing it to be referenced by multiple names.
    aliases: Option<Vec<LitStr>>,
    /// Optional value parser for the argument, allowing for custom parsing logic.
    /// Only supports expression path.
    value_parser: Option<Result<ExprPath, Error>>,
    /// Catch-all fields used to absorb ignored clap attributes.
    /// This prevents darling from failing when encountering unsupported args.
    #[allow(dead_code)]
    #[darling(flatten)]
    ignored_args: Result<(), Error>,
}

impl Default for ClapArg {
    fn default() -> Self {
        Self {
            short: None,
            long: None,
            alias: None,
            aliases: None,
            value_parser: None,
            ignored_args: Ok(()),
        }
    }
}

impl ClapArg {
    /// Creates a `ClapArgAttribute` from an `Attribute`, extracting relevant information from its
    /// meta.
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        Self::from_meta(&attr.meta).ok()
    }

    /// Returns a vector of all aliases for this argument, combining both `alias` and `aliases` fields.
    pub fn aliases(&self) -> Vec<LitStr> {
        self.aliases
            .iter()
            .flatten()
            .chain(&self.alias)
            .cloned()
            .collect()
    }

    /// Returns the parsed `value_parser` expression path if available.
    pub fn value_parser(&self) -> Option<ExprPath> {
        self.value_parser.clone().transpose().unwrap_or_default()
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
    pub fn sanitize(attr: &Attribute, denied_args: &[&str]) -> Attribute {
        if attr.path().is_ident("arg")
            && let Meta::List(list) = &attr.meta
            && let Some(args) = list
                .parse_args_with(Punctuated::<Expr, Token![,]>::parse_terminated)
                .ok()
        {
            let sanitized_attrs: Punctuated<Expr, Token![,]> = args
                .into_iter()
                .filter(|expr| {
                    !expr.ident().is_none_or(|ident| {
                        let arg = ident.to_string();

                        denied_args
                            .iter()
                            .any(|denied_arg| arg.starts_with(denied_arg))
                    })
                })
                .collect();

            parse_quote!(#[arg(#sanitized_attrs)])
        } else {
            attr.clone()
        }
    }
}
