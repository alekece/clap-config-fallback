use std::{ffi::OsString, path::PathBuf};

pub fn path(value: PathBuf) -> String {
    value.display().to_string()
}

pub trait FormatArg<T> {
    fn format(&self, value: T) -> OsString;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultFormat;

impl<T> FormatArg<T> for DefaultFormat
where
    T: ToString,
{
    fn format(&self, value: T) -> OsString {
        value.to_string().into()
    }
}

pub struct CustomFormat<F>(pub F);

impl<T, F, R> FormatArg<T> for CustomFormat<F>
where
    F: Fn(T) -> R,
    R: ToString,
{
    fn format(&self, value: T) -> OsString {
        (self.0)(value).to_string().into()
    }
}
