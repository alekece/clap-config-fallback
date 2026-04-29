use syn::{Expr, GenericArgument, Ident, PathArguments, Type, parse_quote};

/// Extension trait for `syn::Type`.
pub trait TypeExt {
    /// Extracts the identifier from a type.
    fn ident(&self) -> Option<&Ident>;
    /// Checks if the type is an `Option` of a specific identifier.
    /// Checks if the type is a specific identifier.
    fn is(&self, ident: &str) -> bool;

    /// Checks if the type is an `Option` of a specific identifier.
    fn is_option_of(&self, ident: &str) -> bool;

    /// Attempts to unwrap an `Option` type, returning the inner type if it is an `Option`,
    /// or `None` otherwise.
    fn unwrap_option(&self) -> &Self;

    /// Converts the type to an `Option` type if it is not already an `Option`.
    fn to_option(&self) -> Type;

    fn is_unit(&self) -> bool;
}

impl TypeExt for Type {
    fn ident(&self) -> Option<&Ident> {
        match self {
            Type::Path(type_path) => type_path.path.segments.last().map(|segment| &segment.ident),
            _ => None,
        }
    }

    fn is_unit(&self) -> bool {
        matches!(self, Type::Tuple(tuple) if tuple.elems.is_empty())
    }

    fn is(&self, ident: &str) -> bool {
        if let Type::Path(type_path) = self
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == ident
        {
            true
        } else {
            false
        }
    }

    fn is_option_of(&self, ident: &str) -> bool {
        if let Type::Path(type_path) = self
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
            && let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(ty)) = args.args.first()
        {
            ty.is(ident)
        } else {
            false
        }
    }

    fn unwrap_option(&self) -> &Self {
        if let Type::Path(type_path) = self
            && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Option"
            && let PathArguments::AngleBracketed(args) = &segment.arguments
            && let Some(GenericArgument::Type(ty)) = args.args.first()
        {
            ty
        } else {
            self
        }
    }

    fn to_option(&self) -> Type {
        if self.is("Option") {
            self.clone()
        } else {
            parse_quote! { Option<#self> }
        }
    }
}

/// Extension trait for `syn::Expr`.
pub trait ExprExt {
    /// Attempts to extract an identifier from the expression.
    fn ident(&self) -> Option<&Ident>;
}

impl ExprExt for Expr {
    fn ident(&self) -> Option<&Ident> {
        match self {
            Expr::Path(p) => p.path.get_ident(),
            Expr::Assign(a) => match &*a.left {
                Expr::Path(p) => p.path.get_ident(),
                _ => None,
            },
            Expr::Call(c) => match &*c.func {
                Expr::Path(p) => p.path.get_ident(),
                _ => None,
            },
            _ => None,
        }
    }
}
