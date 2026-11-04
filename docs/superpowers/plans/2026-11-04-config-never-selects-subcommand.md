# Config Never Selects the Subcommand Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The CLI alone selects the subcommand; the config file only supplies parameters for all commands and nested subcommands.

**Architecture:** `ConfigSubcommand` generates the config type as a struct of optional per-variant fields (`deny_unknown_fields`) instead of a serde enum. After the whole file is deserialized and before the Figment merge, the config is pruned to the variant the CLI selected (recursively); with no CLI command, command sections are dropped. Merge and clap re-parse are unchanged.

**Tech Stack:** Rust (MSRV 1.89), clap, serde, figment, syn/quote proc-macros.

**Spec:** `docs/designs/2026-11-04-config-never-selects-subcommand-design.md` (issue #15)

## Global Constraints

- clap stays the final parser and error reporter; `clap::Error` stays at the API boundary.
- Precedence stays CLI > config; config never selects or promotes a command.
- Unknown or `#[config(skip)]` command sections are config errors (`ErrorKind::InvalidValue`, "invalid configuration file"), even for unselected commands.
- Prefer no new public trait; any hidden helper is `#[doc(hidden)]`.
- Remove `#[config(tag = "...")]` and selection by config (`command = "serve"`), breaking; both crates go to 0.2.0.
- Config shape: `[command.build]`, `[command.build.command.server]`, same in TOML/JSON/YAML.
- Respect repo rules in `AGENTS.md` (import style, docs, checks).

## Review Focus

- Two command sections with CLI choosing each one: the CLI choice wins, other section ignored.
- Nested `build server` with sections for `build`, `server` and a sibling: only the selected branch applies.
- No CLI command but config sections: clap's usual missing-subcommand error, not a promoted command.
- Unknown section name while CLI selects another command: error.
- Ill-typed section in an unselected command: error.
- Alias keys (`#[config(aliases = [...])]` on the subcommand field, e.g. `cmd`, `action`) still work.

## Tasks

### Task 1: Regression tests (red)

**Files:**
- Modify: `tests/subcommand.rs`

- [ ] **Step 1:** Remove `empty_variant_is_loaded_from_config` and `missing_config_tag_returns_error` (selection by config no longer exists). Keep the other existing tests; rename `unknown_config_tag_returns_error` / `skipped_config_variant_returns_error` to `unknown_command_section_returns_error` / `skipped_command_section_returns_error`.
- [ ] **Step 2:** Rework `newtype_variant_is_loaded_from_config` and `struct_variant_is_loaded_from_config` to pass the command on the CLI (`"build"`, `"debug"`), asserting config values still fill its parameters (aliases `cmd`/`action` still used).
- [ ] **Step 3:** Add tests, one per Review Focus line: `cli_command_wins_with_multiple_sections` (sections `build` + `debug`, run each CLI choice), `no_cli_command_with_sections_returns_error`, `unknown_section_errors_when_other_command_selected`, `ill_typed_unselected_section_returns_error`, `nested_subcommand_uses_selected_branch` (new nested enum `Server`/`Client` under `Build`, asserting values for `build server` and that sibling sections are ignored).
- [ ] **Step 4:** Run `cargo test --all-features --test subcommand`. Expected: the new multi-section/nested tests FAIL.
- [ ] **Step 5:** Commit `test: cover config never selecting the subcommand`.

### Task 2: Generate the config struct and prune to the CLI branch

**Files:**
- Modify: `clap-config-fallback-derive/src/generator/enum.rs`, `clap-config-fallback-derive/src/generator/struct.rs` (where a `#[command(subcommand)]` field is handled), `src/lib.rs`

**Interfaces:**
- Produces: a `#[doc(hidden)]` hook on `ConfigFallback`, e.g. `fn prune_config(config: Self::Config, selected: &Self::Opts) -> Self::Config` (default: identity). Generated enum code keeps only the selected variant's field (recursing into newtype variants' own `prune_config`); generated struct code applies it to its subcommand field.

- [ ] **Step 1:** In `generate_enum` for `GenerationTarget::Config`, emit a struct `{Ident}Config` with `#[serde(rename_all = "snake_case", deny_unknown_fields)]`: one `Option<…>` field per non-skipped variant (unit → empty section, newtype → `Option<<Ty as ConfigFallback>::Config>`, struct → `Option<per-variant struct>`), `skip_serializing_if = "Option::is_none"`. Keep the `Opts` enum generation as is.
- [ ] **Step 2:** Implement the pruning hook as described, and call it in `try_parse_with_config_from` after config load and before the Figment join, passing the parsed `opts`. With no CLI command, the hook drops all command fields.
- [ ] **Step 3:** Run `cargo test --all-features --test subcommand`. Expected: all PASS. Keep generated paths fully qualified.
- [ ] **Step 4:** Commit `fix: let only the CLI select the subcommand`.

### Task 3: Remove `tag`

**Files:**
- Modify: `clap-config-fallback-derive/src/derive/config_subcommand.rs`, `clap-config-fallback-derive/src/generator/enum.rs` (drop `EnumLike::tag` and `tag_attr`), anything else using `tag`.

- [ ] **Step 1:** Find uses: `rg "tag" --glob '!target'`.
- [ ] **Step 2:** Remove the attribute, its parsing, docs and uses.
- [ ] **Step 3:** Run `cargo check --workspace --all-targets --all-features --locked`. Expected: clean.
- [ ] **Step 4:** Commit `feat!: remove config tag attribute`.

### Task 4: Docs, examples, version

**Files:**
- Modify: `README.md`, `examples/subcommand.rs`, `examples/config.toml`, `examples/config.json`, `examples/value_format.rs` (if affected), `Cargo.toml` (`[workspace.package] version` and the derive dependency `version`), `Cargo.lock`

- [ ] **Step 1:** README: new subcommand config shape (with a nested example), "the CLI selects the command" rule, migration section (per design), `tag` removed from the attribute reference.
- [ ] **Step 2:** Update examples to the new shape and run `cargo run --example subcommand --all-features -- --help` plus one with a command.
- [ ] **Step 3:** Bump both crates to 0.2.0 and refresh `Cargo.lock` (`cargo check --workspace --all-features`).
- [ ] **Step 4:** Commit `docs: document subcommand config shape and 0.2.0 migration`.

### Task 5: Full verification

- [ ] **Step 1:** Run the four AGENTS.md checks (fmt, check, clippy `-D warnings`, test, all `--locked`).
- [ ] **Step 2:** Feature checks: `cargo check -p clap-config-fallback --no-default-features`, and each single format feature with `derive`.
- [ ] **Step 3:** `cargo msrv verify -- cargo check --workspace --all-targets --all-features --locked`.
- [ ] **Step 4:** Fresh-agent review against the design; fix material findings.
