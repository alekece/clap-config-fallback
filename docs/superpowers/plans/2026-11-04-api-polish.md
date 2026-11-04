# API Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tidy the public surface and protect diagnostics, feature combinations and docs (#27,
#28, #30, #31).

**Architecture:** Attribute-only changes in `src/lib.rs` for the surface; README becomes the crate
docs; CI gains a feature-powerset and a docs job; `trybuild` covers derive errors.

**Tech Stack:** Rust 1.94.1 (pinned), MSRV 1.89, cargo-hack, trybuild.

**Spec:** `docs/designs/2026-11-04-api-polish-design.md` (sections #27, #28, #30, #31)

## Global Constraints

- Stacked on `feat/29-generated-type-names` (#33); pushed as `chore/27-api-polish`, PR says
  `Fixes #27 #28 #30 #31`. Retarget to `main` after #33 merges.
- New dev-dependency must pass `cargo deny check` and `cargo msrv verify`.
- Doc-tested README snippets show their `use` lines; no hidden `# ` lines (they render on GitHub
  and crates.io). Snippets that are not whole items (a lone field) are `ignore`.

## Review Focus

- `cargo test` without the `derive` feature: README doctests need `derive`; gate the
  `include_str!` doc so `cargo test --no-default-features --features toml` still builds.
- Generated code still compiles with the hidden items (all tests).
- trybuild snapshots are stable across Linux, macOS and Windows CI (trybuild normalizes paths).

---

### Task 1: Public surface (#28)

**Files:** `src/lib.rs`

- [ ] `#[doc(hidden)]` on `Arg`, `IntoArgs`, `FromArgs` re-export, `ConfigFallback`, `mod format`;
  `#[non_exhaustive]` on `ConfigFormat`.
- [ ] Full checks pass. Commit `refactor: hide generated-code internals from docs (#28)`.

### Task 2: CI and warnings (#30)

**Files:** `.github/workflows/ci.yaml`, `src/lib.rs` (warnings found)

- [ ] Run `RUSTFLAGS=-Dwarnings cargo hack check --feature-powerset --lib -p clap-config-fallback
  --locked`; fix each warning (e.g. unused `content` with no format).
- [ ] Run `RUSTDOCFLAGS=-Dwarnings cargo doc --workspace --no-deps --all-features --locked`; fix.
- [ ] Add a `features` job (cargo-hack via `taiki-e/install-action`) and a `docs` job running the
  two commands above; `build` needs them.
- [ ] Commit `ci: check feature combinations and docs (#30)`.

### Task 3: Crate docs from README (#31)

**Files:** `src/lib.rs`, `README.md`

- [ ] Replace the `//!` block with `#![cfg_attr(feature = "derive", doc = include_str!("../README.md"))]`
  and a short fallback `//!` line for non-derive builds.
- [ ] Make every README `rust` block compile: add visible `use` lines; `ignore` only lone fields.
- [ ] README: a missing config file is an error, even when the path comes from `default_value`;
  make the path optional (no default) to make the config optional.
- [ ] `cargo test --doc --all-features --locked` passes; docs job command passes.
- [ ] Commit `docs: build crate docs from the README (#31)`.

### Task 4: Compile-fail tests (#27)

**Files:** `Cargo.toml` (`trybuild` dev-dep), `tests/compile_fail.rs`, `tests/ui/*.rs` + `.stderr`

- [ ] One case per diagnostic: tuple field, alias on non-command field, alias on flattened field,
  `#[config(path)]` wrong type, tuple enum variant, non-literal `name`, unsupported `rename_all`,
  invalid generated type name.
- [ ] Generate snapshots with `TRYBUILD=overwrite`, review each `.stderr` reads well.
- [ ] `cargo deny check`, `cargo msrv verify -- cargo check --workspace --all-targets --all-features
  --locked`, full checks pass.
- [ ] Commit `test: cover derive diagnostics with trybuild (#27)`.
