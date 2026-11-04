# Generated Type Names Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let users choose the names of the generated `Opts` and `Config` types (#29).

**Architecture:** New optional type-level `#[config(opts = "...", config = "...")]` keys, parsed by
darling as `Ident`, replace the default `<Name><Suffix>` ident in the struct and enum generators.
Nested types already link through `<T as ConfigFallback>::Opts/Config`, so nothing else changes.

**Tech Stack:** Rust, darling 0.21, syn, quote.

**Spec:** `docs/designs/2026-11-04-api-polish-design.md` (section #29)

## Global Constraints

- Defaults stay `<Name>Opts` and `<Name>Config`; existing users see no change.
- Keys are string literals, spelled `opts` and `config`.
- An invalid identifier is a compile error on that value (darling's `Ident` parsing provides it).
- Branch pushed as `feat/29-generated-type-names`; PR says `Fixes #29`.

## Review Focus

- A user type named `<Name>Config` next to the derive: compiles once `config = "..."` is set.
- A renamed nested `ConfigArgs` inside a `ConfigParser`: still merges CLI and config.
- A renamed `ConfigSubcommand`: still selects and fills the subcommand.
- Only one of the two keys set: the other keeps its default name.

---

### Task 1: `opts` / `config` keys

**Files:**
- Modify: `clap-config-fallback-derive/src/derive/config_parser.rs`, `config_subcommand.rs`,
  `config_args.rs`
- Modify: `clap-config-fallback-derive/src/generator/struct.rs:15-20,123`,
  `generator/enum.rs:15-20,75,137`
- Test: `tests/generated_names.rs`

**Interfaces:**
- Produces: `StructLike::generated_ident(&self, target: GenerationTarget) -> Ident` and the same on
  `EnumLike`: the custom name for the target, else `format_ident!("{}{}", ident, target.suffix())`.
  Both darling inputs gain `#[darling(default)] opts: Option<Ident>` and `config: Option<Ident>`.

- [ ] **Step 1: Write the failing tests** in `tests/generated_names.rs`:
  - `custom_names_avoid_clashes`: a user `struct CliConfig;` plus
    `#[config(opts = "CliArgsOpts", config = "CliFile")] struct Cli` with a nested renamed
    `ConfigArgs` and a renamed `ConfigSubcommand`; parse with a TOML file and CLI args and assert
    merged values; also assert `CliFile` and `CliArgsOpts` are nameable types
    (`let _: Option<CliFile> = None;`).
  - `one_key_keeps_other_default`: only `opts = "X"` set; `<Name>Config` still exists.
- [ ] **Step 2:** `cargo test --all-features --locked --test generated_names` fails (unknown field
  `opts` / duplicate `CliConfig`).
- [ ] **Step 3:** Add the fields and `generated_ident`; use it at the three `format_ident!` sites.
- [ ] **Step 4:** The test passes; full checks pass.
- [ ] **Step 5:** Commit `feat: allow naming the generated Opts and Config types (#29)`.

### Task 2: Documentation

**Files:** `README.md` (type-level attribute table and a short "Generated types" section)

- [ ] **Step 1:** Document the default names, why they can clash, and the two keys with an example.
- [ ] **Step 2:** Commit `docs: document generated type names`.
