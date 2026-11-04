# Config round-trip fixes plan

Fixes #18, #19 and #20 from the repository review. These are bug fixes with no public API change, so there is no separate design document.

## Tasks

1. **#18: dash-prefixed values.** In `src/arg.rs`, emit named scalar and repeated values as one `name=value` token, so clap never reads the value as a flag. Positional values keep the current form. Add tests for negative ints, `-x` strings, a short flag and a `Vec`.
2. **#19: clap arg ids.** In the derive crate, compute the clap id once: `#[arg(id = "...")]` if present, otherwise the unraw field name. Use it in the `from_args` initializers. Use the unraw name for inherited `long`/`short`. Add tests for `r#type` and `id = "..."`.
3. **#20: argument order.** Emit plain fields first, then flattened fields, then subcommands, in struct and enum-variant `extend_args`. Add a test with the config path declared after a flattened struct that holds a subcommand.
4. **Examples.** Move the `[server]`/`[database]` keys in `examples/config.toml` and `config.json` to the top level to match the flattened fields.
5. Run the AGENTS.md checks, get a fresh review, and open the PR.
