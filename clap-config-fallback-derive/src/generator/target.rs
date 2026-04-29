use crate::derive::Skippable;

#[derive(Debug, Copy, Clone)]
pub enum GenerationTarget {
    Opts,
    Config,
}

impl GenerationTarget {
    pub fn suffix(&self) -> &'static str {
        match self {
            Self::Opts => "Opts",
            Self::Config => "Config",
        }
    }

    pub fn should_skip<T: Skippable>(&self, value: &T) -> bool {
        match self {
            Self::Opts => false,
            Self::Config => value.is_skipped(),
        }
    }
}
