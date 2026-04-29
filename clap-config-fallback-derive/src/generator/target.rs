use crate::derive::Skippable;

/// Target for code generation.
#[derive(Debug, Copy, Clone)]
pub enum GenerationTarget {
    /// Generate intermediate CLI merge representation.
    Opts,
    /// Generate config-file deserializable representation.
    Config,
}

impl GenerationTarget {
    /// Suffix appended to generated types.
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Opts => "Opts",
            Self::Config => "Config",
        }
    }

    /// Wheither the given value should be skipped during generation for this target.
    pub fn should_skip<T: Skippable>(&self, value: &T) -> bool {
        match self {
            Self::Opts => false,
            Self::Config => value.is_skipped(),
        }
    }
}
