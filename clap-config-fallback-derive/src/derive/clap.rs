use darling::{Error, FromMeta, util::Override};
use syn::{Attribute, Expr, ExprPath, LitStr, Meta, Token, parse_quote, punctuated::Punctuated};

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
    /// Optional override for the clap argument id.
    /// Only supports string literals.
    id: Option<Result<LitStr, Error>>,
    /// Deprecated clap spelling of `id`, still honored by clap.
    name: Option<Result<LitStr, Error>>,
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
    /// Whether `id` comes before `short` in the attribute, in which case clap derives the
    /// inherited short from the id rather than the field name.
    #[darling(skip)]
    id_before_short: bool,
    /// Same as `id_before_short`, for `long`.
    #[darling(skip)]
    id_before_long: bool,
}

impl Default for ClapArg {
    fn default() -> Self {
        Self {
            id: None,
            name: None,
            short: None,
            long: None,
            alias: None,
            aliases: None,
            value_parser: None,
            ignored_args: Ok(()),
            id_before_short: false,
            id_before_long: false,
        }
    }
}

impl ClapArg {
    /// Creates a `ClapArgAttribute` from an `Attribute`, extracting relevant information from its
    /// meta.
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        let mut arg = Self::from_meta(&attr.meta).ok()?;
        let items = Self::parse_items(attr).unwrap_or_default();
        let position = |wanted: &str| {
            items
                .iter()
                .position(|expr| expr.ident().is_some_and(|ident| ident == wanted))
        };
        let id_position = position("id").or_else(|| position("name"));
        let id_before = |flag: &str| {
            matches!(
                (id_position, position(flag)),
                (Some(id), Some(flag)) if id < flag
            )
        };

        arg.id_before_short = id_before("short");
        arg.id_before_long = id_before("long");

        Some(arg)
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

    /// Returns the explicit clap argument id if available.
    pub fn id(&self) -> Option<String> {
        self.id
            .clone()
            .or_else(|| self.name.clone())?
            .ok()
            .map(|id| id.value())
    }

    /// Returns the parsed `value_parser` expression path if available.
    pub fn value_parser(&self) -> Option<ExprPath> {
        self.value_parser.clone().transpose().unwrap_or_default()
    }

    /// Generates the appropriate short or long flag for this argument, inheriting from `name` (the
    /// name clap derives so far: the kebab-case field name or an explicit id) when the flag has no
    /// explicit value.
    ///
    /// Note that if both `short` and `long` are specified, `short` will take precedence for
    /// generating the flag
    pub fn flag_name(&self, name: &str) -> Option<String> {
        let id = self.id();
        let inherited_name = |id_before_flag: bool| {
            id.as_deref()
                .filter(|_| id_before_flag)
                .unwrap_or(name)
                .to_owned()
        };

        if let Some(short) = &self.short {
            return match short {
                Override::Explicit(c) => Some(format!("-{c}")),
                Override::Inherit => inherited_name(self.id_before_short)
                    .chars()
                    .next()
                    .map(|c| format!("-{c}")),
            };
        }

        if let Some(long) = &self.long {
            return match long {
                Override::Explicit(s) => Some(format!("--{s}")),
                Override::Inherit => Some(format!("--{}", inherited_name(self.id_before_long))),
            };
        }

        None
    }

    /// Sanitizes the given attribute by removing any sub-attributes that are not relevant for
    /// optional argument.
    pub fn sanitize(attr: &Attribute, denied_args: &[&str]) -> Attribute {
        if attr.path().is_ident("arg")
            && let Some(args) = Self::parse_items(attr)
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

    /// Parses the comma-separated items of a list attribute such as `#[arg(id = "x", long)]`, in
    /// declaration order.
    fn parse_items(attr: &Attribute) -> Option<Punctuated<Expr, Token![,]>> {
        let Meta::List(list) = &attr.meta else {
            return None;
        };

        list.parse_args_with(Punctuated::parse_terminated).ok()
    }
}
