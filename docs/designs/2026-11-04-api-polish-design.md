# API polish before 0.3.0

Issues: #27, #28, #29, #30, #31. Ships in 0.3.0, which is already breaking.

## Why

The second audit found gaps in the public surface and in what CI protects. Generated types clash
with user types, internals show on docs.rs, the docs.rs page disagrees with the README, and
derive errors, feature combinations and docs are untested.

## Decisions

### #29: name the generated types

- New type-level keys on `ConfigParser`, `ConfigArgs` and `ConfigSubcommand`:
  `#[config(opts = "CliOpts", config = "CliFile")]`. Both are optional; defaults stay
  `<Name>Opts` and `<Name>Config`.
- Nested types already refer to each other through `<T as ConfigFallback>::Opts/Config`, so a
  custom name needs no other change.
- A value that is not a valid Rust identifier is a compile error on that value.
- The README documents the default names and the keys.
- Approaches considered: one key per type (chosen by the user), a shared prefix/suffix, or a
  generated module. One key per type is the most explicit and solves name clashes directly.
- My calls: the key is `opts` (matching the `Opts` suffix); values are string literals, like the
  other `#[config]` keys.

### #28: public surface

- `Arg`, `IntoArgs`, `FromArgs`, `ConfigFallback` and `format` get `#[doc(hidden)]`; they stay
  public because generated code uses them.
- `ConfigFormat` gets `#[non_exhaustive]`.

### #27: derive diagnostics

- `trybuild` compile-fail tests cover the existing derive errors and the new invalid-name error.
- Compiler output changes between versions, so these tests run only on the pinned toolchain
  (`rust-toolchain.toml`) and are skipped elsewhere, e.g. the MSRV job.

### #30: CI

- CI runs `cargo hack check --feature-powerset` on the library, and `cargo doc` with
  `RUSTDOCFLAGS=-Dwarnings`.
- Existing warnings these reveal (such as the unused `content` without formats) are fixed.

### #31: crate docs

- `src/lib.rs` uses `#![doc = include_str!("../README.md")]`, so README examples are doc-tested.
- README examples are fixed to compile; examples that need features are gated or use `ignore`
  only where they cannot run.
- README states that a missing config file is an error, including a `default_value` path; leave
  the path without default to make the config optional.

## Delivery

Two PRs: #29 alone, then #27, #28, #30 and #31 together.

## Non-goals

- Changing the generated types' visibility or hiding them.
- Releasing 0.3.0 (separate step after both PRs).
