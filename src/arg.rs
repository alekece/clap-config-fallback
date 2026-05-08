use std::ffi::OsString;

use clap::ArgMatches;

use crate::format::{CustomFormat, DefaultFormat, FormatArg};

/// Converts an intermediate options struct into synthetic CLI args.
///
/// These args are fed back into clap for the final parse/validation pass.
pub trait IntoArgs: Sized {
    fn into_args(self) -> impl Iterator<Item = OsString> {
        let mut args = Vec::new();

        self.extend_args(&mut args);

        args.into_iter()
    }

    fn extend_args(self, args: &mut Vec<OsString>);
}

/// Builds an intermediate options struct from clap `ArgMatches`.
///
/// This pass captures values explicitly provided on the CLI before config fallback is applied.
pub trait FromArgs: Sized {
    fn from_args(args: &ArgMatches) -> Option<Self>;
}

pub enum ArgValue<T> {
    Scalar(T),
    Repeated(Vec<T>),
    Flag(bool),
}

pub struct Arg<T, F = DefaultFormat> {
    value: ArgValue<T>,
    name: Option<OsString>,
    formatter: F,
}

impl<T> Arg<T> {
    pub fn scalar(value: T) -> Self {
        Self {
            value: ArgValue::Scalar(value),
            name: None,
            formatter: DefaultFormat,
        }
    }

    pub fn repeated(value: Vec<T>) -> Self {
        Self {
            value: ArgValue::Repeated(value),
            name: None,
            formatter: DefaultFormat,
        }
    }
}

impl Arg<bool, DefaultFormat> {
    pub fn flag(value: bool) -> Self {
        Self {
            value: ArgValue::Flag(value),
            name: None,
            formatter: DefaultFormat,
        }
    }
}

impl<T, F> Arg<T, F> {
    pub fn name(mut self, name: impl Into<OsString>) -> Self {
        self.name = Some(name.into());

        self
    }

    pub fn value_format<U>(self, formatter: U) -> Arg<T, CustomFormat<U>> {
        Arg {
            value: self.value,
            name: self.name,
            formatter: CustomFormat(formatter),
        }
    }
}

impl<T, F> IntoArgs for Arg<T, F>
where
    F: FormatArg<T>,
{
    fn extend_args(self, args: &mut Vec<OsString>) {
        match self.value {
            ArgValue::Scalar(value) => {
                if let Some(name) = self.name {
                    args.push(name);
                }

                args.push(self.formatter.format(value));
            }
            ArgValue::Repeated(values) => {
                for value in values {
                    if let Some(name) = &self.name {
                        args.push(name.clone());
                    }

                    args.push(self.formatter.format(value));
                }
            }
            ArgValue::Flag(value) => {
                if value && let Some(name) = self.name {
                    args.push(name);
                }
            }
        }
    }
}

impl<T> IntoArgs for Option<T>
where
    T: IntoArgs,
{
    fn extend_args(self, args: &mut Vec<OsString>) {
        if let Some(value) = self {
            value.extend_args(args);
        }
    }
}
