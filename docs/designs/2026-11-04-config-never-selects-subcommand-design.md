# Config Never Selects the Subcommand

## Intent

`parse_with_config()` lets the config file override the subcommand given on the CLI when the config has sections for several subcommands. The generated config type is an externally tagged serde enum, so merging several command sections produces several variant keys and serde silently picks one. Reported in [#2](https://github.com/alekece/clap-config-fallback/pull/2) by @regexident; tracked by [issue #15](https://github.com/alekece/clap-config-fallback/issues/15).

The root fix is to stop treating the subcommand as something config can select. The CLI alone selects the command; config only supplies parameter defaults, for every command and nested subcommand at once.

## Goals and non-goals

- The subcommand given on the CLI is never overridden by config.
- Config can define parameters for all subcommands simultaneously, including nested ones (`xxx build server`).
- With no subcommand on the CLI, clap reports its usual error; config never promotes a command.
- Fallback precedence for non-command fields stays CLI > config.
- A config section for a non-existent subcommand, or for a `#[config(skip)]` variant, is a configuration error (current behaviour, kept).
- Config sections for commands the CLI did not select are ignored.
- Remove `#[config(tag = "...")]` and `command = "serve"` style selection (breaking), and document the migration in the README.
- Bump the crate version to 0.2.0.
- Non-goals: no `fallback_command` or any other way for config to select a command; no opt-in compatibility mode; no change to flattened structs or non-command behaviour.

## Constraints

- clap stays the final parser, validator and error reporter; `clap::Error` stays at the API boundary.
- Library and derive crate are one public contract; both change together, and generated code must keep working with the minimum supported Rust version (1.89).
- Config file shape is the same in TOML, JSON and YAML.
- Mechanism today: the CLI is parsed into a generated `Opts` value, merged with the config through Figment (CLI wins), extracted back into `Opts`, converted to args and re-parsed by clap. The command is an enum variant on the `Opts` side.

## Approaches

- **Struct-of-optional-variants config, with config pruned to the CLI-selected branch before the merge (chosen).** `ConfigSubcommand` generates a config struct with one optional field per variant (kebab-case name, `deny_unknown_fields` so unknown names error). Before the Figment merge, the config is reduced to only the variant the CLI selected, recursively for nested subcommands; with no CLI selection, all command sections are dropped. The merge itself stays as it is.
- Extract and re-inject the command around the Figment merge (the approach in PR #2, minus `fallback_command`): works, but adds two public traits and a second merge path.
- Keep the enum and add an ambiguity check: rejected; it cannot fix the bug, since multiple sections are legitimate under the new model.
- Write fresh rather than adapting PR #2 (maintainer's choice); credit the reporter in the PR description.

## The config shape

```toml
[command.build]            # `build` parameters
target = "x86_64-unknown-linux-gnu"

[command.build.command.server]   # nested subcommand `server` of `build`
port = 8080

[command.run]
logging = true
```

Running `xxx build server` uses the `build` and `server` values; `xxx run` uses `logging`; `xxx` alone is a clap error. Unit variants have no parameters and need no section. A section naming an unknown or skipped command, or with invalid values, is a configuration error even when the CLI selects another command, because the whole file is validated as today.

## Migration (documented in the README)

- `command = "serve"` and `command = { debug = { ... } }` (selection by config) → no longer supported; select the command on the CLI.
- `[command]` + `#[config(tag = "name")]` internally tagged form → remove the attribute; move parameters into `[command.<variant>]`.
- Remove the corresponding README sections and attribute-reference entry; update examples and tests that use selection or `tag`.

## Decisions made without the maintainer (my calls)

- Unknown and skipped variant names error, as today and as in PR #2. A skipped variant has no field in the config struct, so it fails as an unknown name.
- The whole config file is deserialized into the typed config struct first, exactly as today, and the error stays a `clap::Error` (`ErrorKind::InvalidValue`, "invalid configuration file"). So an ill-typed or unknown section is an error even for commands the CLI did not select; pruning happens afterwards on the typed value. Well-formed sections for unselected commands are then ignored.
- `[command.build]` for a single command already works today, so the section shape is unchanged; the breaking part is only the removal of selection by config and of `tag`, plus the now-legitimate coexistence of several sections.
- Optional and required subcommands behave the same: config never promotes a command, and with no CLI command all command sections are dropped. No existing test or example uses an optional subcommand.
- Unit, newtype and struct variants are all covered by the existing tests and must stay covered.
- Both crates move to 0.2.0 together, including the derive crate's version pin in the root `Cargo.toml` and `Cargo.lock`. There is no changelog; the README migration section is the release note.
- Section names are kebab-case, matching the CLI command names (maintainer's choice in PR review). This supersedes the initial snake_case choice: multi-word sections such as `[command.dry_run]` become `[command.dry-run]`, documented in the README migration notes.
- The pruning mechanism (above) is proposed, not required; any approach meeting the goals is acceptable.

## Left to the builder

- Naming and shape of any new trait or generated helper (keep the public surface minimal; prefer no new public trait).
- How unit variants and struct variants (not only newtype variants) are represented in the config struct.
- Test layout, as long as the regression tests cover: multiple command sections with each CLI choice, nested subcommands, no CLI command with config sections, and the unknown and skipped cases.
