use darling::{Error, FromMeta, util::Override};
use heck::{ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToUpperCamelCase};
use syn::{
    Attribute, Expr, ExprLit, ExprPath, Ident, Lit, LitStr, Meta, Token, parse_quote,
    punctuated::Punctuated,
};

use crate::syn_utils::ExprExt;

#[derive(Copy, Clone, PartialEq, Eq, FromMeta)]
pub enum ClapCommand {
    Flatten,
    Subcommand,
}

/// Casing clap applies to derived names, set with `#[command(rename_all = "...")]`.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub enum RenameRule {
    Camel,
    #[default]
    Kebab,
    Pascal,
    ScreamingSnake,
    Snake,
    Lower,
    Upper,
    Verbatim,
}

/// The type- or variant-level `#[command(...)]` items that change the names clap derives.
#[derive(Clone, Default)]
pub struct ClapCommandAttr {
    name: Option<String>,
    rename_all: Option<RenameRule>,
}

impl ClapCommand {
    pub fn from_attr(attr: &Attribute) -> Option<Self> {
        Self::from_meta(&attr.meta).ok()
    }
}

impl RenameRule {
    /// Parses a casing with clap's normalization, so `"kebab-case"`, `"kebab"` and `"KebabCase"`
    /// are all accepted.
    fn parse(value: &str) -> Option<Self> {
        let rule = match value.to_upper_camel_case().to_lowercase().as_str() {
            "camel" | "camelcase" => Self::Camel,
            "kebab" | "kebabcase" => Self::Kebab,
            "pascal" | "pascalcase" => Self::Pascal,
            "screamingsnake" | "screamingsnakecase" => Self::ScreamingSnake,
            "snake" | "snakecase" => Self::Snake,
            "lower" | "lowercase" => Self::Lower,
            "upper" | "uppercase" => Self::Upper,
            "verbatim" | "verbatimcase" => Self::Verbatim,
            _ => return None,
        };

        Some(rule)
    }

    /// Converts a Rust identifier to the name clap derives from it, mirroring clap's
    /// `Name::translate`.
    pub fn apply(self, name: &str) -> String {
        let name = name.strip_prefix("r#").unwrap_or(name);

        match self {
            Self::Camel => name.to_lower_camel_case(),
            Self::Kebab => name.to_kebab_case(),
            Self::Pascal => name.to_upper_camel_case(),
            Self::ScreamingSnake => name.to_shouty_snake_case(),
            Self::Snake => name.to_snake_case(),
            Self::Lower => name.to_snake_case().replace('_', ""),
            Self::Upper => name.to_shouty_snake_case().replace('_', ""),
            Self::Verbatim => name.to_owned(),
        }
    }
}

impl ClapCommandAttr {
    /// Collects `name` and `rename_all` from `#[command(...)]` attributes, ignoring other items.
    /// A later value overrides an earlier one, as in clap.
    ///
    /// Only string literals are supported: generated code must know these names at compile time to
    /// produce matching arguments and config sections.
    pub fn from_attrs(attrs: &[Attribute]) -> Result<Self, Error> {
        let mut command = Self::default();
        let mut error = Error::accumulator();

        for attr in attrs.iter().filter(|attr| attr.path().is_ident("command")) {
            for expr in ClapArg::parse_items(attr).unwrap_or_default() {
                let Expr::Assign(assign) = &expr else {
                    continue;
                };

                let Some(key) = expr.ident().map(Ident::to_string) else {
                    continue;
                };

                if key != "name" && key != "rename_all" {
                    continue;
                }

                let value = match &*assign.right {
                    Expr::Lit(ExprLit {
                        lit: Lit::Str(value),
                        ..
                    }) => value,
                    right => {
                        error.push(
                            Error::custom(format!(
                                "`{key}` must be a string literal to be used with config fallback"
                            ))
                            .with_span(right),
                        );

                        continue;
                    }
                };

                if key == "name" {
                    command.name = Some(value.value());
                } else if let Some(rule) = RenameRule::parse(&value.value()) {
                    command.rename_all = Some(rule);
                } else {
                    error.push(Error::custom("unsupported `rename_all` value").with_span(value));
                }
            }
        }

        error.finish_with(command)
    }

    /// Returns the explicit command name, if any.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the explicit casing, if any.
    pub fn rename_all(&self) -> Option<RenameRule> {
        self.rename_all
    }

    /// Relaxes items that must not apply to the first, all-optional parse, where config values are
    /// not merged yet: `arg_required_else_help` is dropped and argument groups become optional.
    /// The final parse still enforces them.
    pub fn sanitize(attr: &Attribute) -> Attribute {
        let Some(items) = ClapArg::parse_items(attr).filter(|_| attr.path().is_ident("command"))
        else {
            return attr.clone();
        };

        let items: Punctuated<Expr, Token![,]> = items
            .into_iter()
            .filter_map(|expr| {
                let Some(ident) = expr.ident().map(Ident::to_string) else {
                    return Some(expr);
                };

                // clap accepts both `group(value)` and `group = value`.
                let value = match &expr {
                    Expr::Call(call) if call.args.len() == 1 => Some(&call.args[0]),
                    Expr::Assign(assign) => Some(&*assign.right),
                    _ => None,
                };

                match (ident.as_str(), value) {
                    ("arg_required_else_help", _) => None,
                    ("group", Some(group)) => Some(parse_quote!(group(
                        ::clap::ArgGroup::from(#group).required(false)
                    ))),
                    ("groups", Some(groups)) => {
                        Some(parse_quote!(groups((#groups).into_iter().map(|group| {
                            ::clap::ArgGroup::from(group).required(false)
                        }))))
                    }
                    _ => Some(expr),
                }
            })
            .collect();

        parse_quote!(#[command(#items)])
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
