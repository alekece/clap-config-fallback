# Audit Bug Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the bugs found by the 2026-11-04 audit: #22, #23, #24 and #25.

**Architecture:** The derive keeps generating `<Name>Opts`/`<Name>Config`. It now forwards type- and variant-level `#[command(...)]` attributes to `Opts` and derives every CLI name (flags, subcommands, config sections) from the same clap naming rules. The library reaches serde through a hidden re-export, detects help/version from clap's own first-pass result, and reports config errors with the path and without echoing source lines.

**Tech Stack:** Rust 2024 (MSRV 1.89), clap 4 derive, darling, syn/quote, heck, serde, figment, toml, noyalib, serde_json.

**Spec:** issues #22–#25. These are bug fixes, so there is no separate design document. #26 is deferred (see its comment): non-UTF-8 paths fail in the serde merge, not in the formatter.

## Global Constraints

- Clap stays the final parser, validator and error reporter. Precedence and config shape stay unchanged, except that renamed subcommands use their CLI name as the config section name (the README already says sections are CLI command names).
- Generated code uses fully qualified paths only.
- No new runtime dependencies. MSRV is 1.89.
- Public API: add only `#[doc(hidden)] pub mod reexports`.
- Checks: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`.

## Review Focus

- `#[command(rename_all = "snake_case")]` on a struct with a config value: the round-trip flag must be `--max_conn`, not `--max-conn` (Task 2 test `struct_rename_all_round_trips`).
- A variant with `#[command(name = "go")]` and a config section `[cmd.go]`: `app go` works and uses the section (Task 2 test `renamed_variant_uses_cli_name`).
- An enum with `rename_all` and a struct variant whose fields inherit it (Task 2 test `enum_rename_all_applies_to_variant_fields`).
- `-h` as a user flag while a config supplies other values (Task 3 test `short_h_user_flag_keeps_config`).
- A TOML type error on a secret-looking value must not print the source line (Task 4 test `toml_error_hides_source_line`).

---

### Task 1: Generated code no longer needs a user serde dependency (#22)

**Files:**
- Modify: `src/lib.rs` (add the module)
- Modify: `clap-config-fallback-derive/src/generator/{struct.rs,enum.rs,helpers.rs}`: every `::serde::` path
- Create: `tests/no-serde/Cargo.toml`, `tests/no-serde/src/main.rs`
- Modify: root `Cargo.toml` (`[workspace] members`), `Cargo.lock`

**Interfaces:**
- Produces: `clap_config_fallback::reexports::serde` (`#[doc(hidden)] pub mod reexports { pub use serde; }`).

- [ ] **Step 1: Add the regression crate.** `tests/no-serde` is package `no-serde-check` with `publish = false`. Its only dependencies are `clap = { version = "4.6.0", features = ["derive"] }` and `clap-config-fallback = { path = "../.." }`. `main.rs` holds the README's root example: a `Cli` with a path field, `host`/`port` and a `Command` subcommand enum that has one struct variant and one field with `value_parser`, so the deserialize fns are generated. Add `"tests/no-serde"` to workspace `members`. Cargo does not auto-discover it as an integration test because it has no `main.rs` at the `tests/no-serde/` root, and it is not packaged because it is a nested package.
- [ ] **Step 2: Verify it fails.** `cargo check -p no-serde-check`. Expected: "cannot find attribute `serde`".
- [ ] **Step 3: Implement.** Add the `reexports` module. In generated code, replace `::serde::` with `::clap_config_fallback::reexports::serde::`. Add `#[serde(crate = "::clap_config_fallback::reexports::serde")]` after every `#[derive(... Serialize, ... Deserialize)]`: the Opts/Config structs, the Opts enum and the section structs.
- [ ] **Step 4: Verify.** The Step 2 command now succeeds, and the full checks pass.
- [ ] **Step 5: Commit.** `fix: reach serde through clap_config_fallback::reexports (#22)`

### Task 2: Forward type/variant `#[command]` attributes and follow clap naming (#23)

**Files:**
- Modify: `clap-config-fallback-derive/src/derive/clap.rs`: new `ClapCommandAttr` and `RenameRule`
- Modify: `clap-config-fallback-derive/src/derive/{config_parser.rs,config_args.rs,config_subcommand.rs,variant.rs}`
- Modify: `clap-config-fallback-derive/src/generator/{struct.rs,enum.rs,helpers.rs}`
- Test: `tests/command_attrs.rs`

**Interfaces:**
- Produces, in `derive/clap.rs`:
  - `enum RenameRule { Camel, Kebab, Pascal, ScreamingSnake, Snake, Lower, Upper, Verbatim }`, parsed from clap's strings `"camelCase"`, `"kebab-case"`, `"PascalCase"`, `"SCREAMING_SNAKE_CASE"`, `"snake_case"`, `"lower"`, `"UPPER"`, `"verbatim"`. Has `Default = Kebab` and `fn apply(self, name: &str) -> String` (heck, or `to_lowercase`/`to_uppercase`/identity). An unknown string is a darling error.
  - `struct ClapCommandAttr { name: Option<LitStr>, rename_all: Option<RenameRule> }`. It is built from forwarded `#[command(...)]` attributes and ignores other items, like `ClapArg::ignored_args`. `fn from_attrs(attrs: &[Attribute]) -> Self` merges them, with the last value winning.
- `ConfigParser` and `Variant` gain `forward_attrs(command)` → `attrs: Vec<Attribute>`. `ConfigSubcommand` gains the same.
- `StructLike` gains `fn attributes(&self) -> &[Attribute]` and `fn rename_rule(&self) -> RenameRule`.
- `Variant` gains `fn attributes(&self) -> &[Attribute]`, `fn command_name(&self, enum_rule: RenameRule) -> String` (explicit `name`, otherwise `enum_rule.apply(ident)`) and `fn rename_rule(&self, enum_rule: RenameRule) -> RenameRule` (its own `rename_all`, otherwise `enum_rule`).
- `helpers::generate_extend_args_statement(ident, commands_ident, field, rule: RenameRule)`. `generate_flag_name(field, rule)` starts from `rule.apply(unraw field name)` instead of kebab-case. Explicit ids stay verbatim, matching clap's handling of assigned names.

- [ ] **Step 1: Write failing tests** in `tests/command_attrs.rs`. Each test uses a TOML config file:
  - `struct_rename_all_round_trips`: `#[command(rename_all = "snake_case")]`, field `max_conn: u16`. `--max_conn 3` alone gives 3. With `max_conn = 4` in the config and no CLI value, the result is 4.
  - `disable_help_flag_with_custom_short_h`: `#[command(disable_help_flag = true)]` plus `#[arg(short = 'h', long)] host: String`. `-h x` with `port = 9` from the config succeeds (no panic).
  - `renamed_variant_uses_cli_name`: variant `#[command(name = "go")] Run { #[arg(long)] n: u8 }` with config `[cmd.go]\nn = 3`. `app go` gives `n == 3`.
  - `enum_rename_all_applies_to_variant_fields`: enum `#[command(rename_all = "snake_case")]`, variant `DryRun { #[arg(long)] max_conn: u8 }`. `app dry_run --max_conn 2` parses. Config section `[cmd.dry_run]` with `max_conn = 5` and `app dry_run` gives 5.
- [ ] **Step 2: Verify they fail.** `cargo test --test command_attrs`. The current results are "unexpected argument", a debug-assert panic and "unrecognized subcommand".
- [ ] **Step 3: Implement.**
  - Emit the forwarded type attributes on the Opts struct and enum, and the variant attributes on the Opts variants.
  - Use `command_name` everywhere a variant name is produced today: the Opts `#[serde(rename)]`, the config section `rename` (alias = snake_case of the name when it differs), `extend_args`, and the `from_args` match.
  - Pass the struct rule (struct fields) or the variant rule (struct-variant fields) into `generate_extend_args_statement`.
- [ ] **Step 4: Verify.** The new tests and the full checks pass.
- [ ] **Step 5: Commit.** `fix: forward type and variant #[command] attributes (#23)`

### Task 3: Detect help/version from clap's first pass (#24)

**Files:**
- Modify: `src/lib.rs` (`try_parse_with_config_from`)
- Test: `tests/help_version.rs`

- [ ] **Step 1: Write failing tests.**
  - `short_h_user_flag_keeps_config`: the struct uses `disable_help_flag`, `-h`=`--host`, and `port` comes from the config. `["app", "-h", "x", "--config", path]` gives `port == 9`.
  - `help_uses_user_command`: a doc comment `/// My tool about` on the root. `--help` gives `ErrorKind::DisplayHelp`, and `err.to_string()` contains "My tool about".
  - `version_flag`: `#[command(version = "1.2.3")]`. `-V` gives `ErrorKind::DisplayVersion`, and the output contains "1.2.3".
- [ ] **Step 2: Verify that `short_h_user_flag_keeps_config` fails.** It still fails after Task 2, because the scan short-circuits.
- [ ] **Step 3: Implement.** Delete the arg scan. Run the first pass on `args.clone()`. On `ErrorKind::DisplayHelp | DisplayVersion | DisplayHelpOnMissingArgumentOrSubcommand`, return `Self::try_parse_from(args)`. Propagate every other error.
- [ ] **Step 4: Verify.** The tests and the full checks pass.
- [ ] **Step 5: Commit.** `fix: detect help and version from clap instead of scanning args (#24)`

### Task 4: Clear config file errors (#25)

**Files:**
- Modify: `src/lib.rs` (config loading)
- Test: `tests/config_formats.rs`

- [ ] **Step 1: Write failing tests.** Use a root struct without `format = ...` (inference), fields `port: u16` and `token: Option<String>`. Create files with `tempfile::Builder::new().suffix(...)`.
  - `toml_yaml_yml_json_inferred_from_extension`: each suffix (`.toml`, `.yaml`, `.yml`, `.json`) loads `port`.
  - `unknown_extension_is_rejected`: `.conf` gives `InvalidValue`, and the message contains "unsupported format".
  - `missing_file_names_path`: gives `Io`, and the message contains the file name.
  - `yaml_type_error_is_readable`: `token: [x]` gives a message containing "line 2" and not `$__noyalib`.
  - `toml_error_hides_source_line`: `port = 1\ntoken = 123456789` gives a message containing "line 2" and not `token = `.
- [ ] **Step 2: Verify that the missing-path, YAML and TOML tests fail.**
- [ ] **Step 3: Implement.**
  - Remove the `path.exists()` check. The read error becomes `failed to read configuration file {path}: {e}` with `ErrorKind::Io`.
  - YAML uses `noyalib::from_str`.
  - TOML maps errors to `line {line}, column {column}: {e.message()}`, with line and column computed from `e.span()` over `content`, or to `e.message()` when there is no span.
  - The invalid-file message gets the path as well: `invalid configuration file {path}: {e}`.
- [ ] **Step 4: Verify.** The tests and the full checks pass. `missing_config_file_is_reported_as_io_error` still passes.
- [ ] **Step 5: Commit.** `fix: name the config path and avoid echoing config lines in errors (#25)`

### Task 5: Docs and release prep

- [ ] Update the README:
  - Installation no longer needs serde; state that.
  - Type- and variant-level `#[command]` attributes such as `rename_all` and `name` are honored.
  - The section name for a renamed variant is its CLI name.
- [ ] Update the crate docs if they are affected. Run the full checks. Fresh independent review. Open the PR with `Fixes #22 #23 #24 #25`.
