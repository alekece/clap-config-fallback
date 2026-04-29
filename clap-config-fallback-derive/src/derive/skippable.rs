/// Shared interface for derive nodes that can be skipped during code generation.
pub trait Skippable {
    /// Returns whether this value is excluded from generated configuration types.
    fn is_skipped(&self) -> bool;
}
