# Config round-trip fixes plan

Fixes #18, #19 and #20 from the repository review. These are bug fixes with no public API change, so there is no separate design document.

## Tasks

1. **#18: dash-prefixed values.** In `src/arg.rs`, emit named scalar and repeated values as one `name=value` token, so clap never reads the value as a flag. Positional values keep the current form. Add tests for negative ints, `-x` strings, a short flag and a `Vec`.
2. **#19: clap arg ids.** In the derive crate, compute the clap id once: `#[arg(id = "...")]` if present, otherwise the unraw field name. Use it in the `from_args` initializers. Inherited `long`/`short` follow clap's attribute order: the kebab-case unraw field name, or an `id` (or deprecated `name`) declared before the flag. Add tests for `r#type` and `id = "..."`.
3. **#20: argument order.** Add a `#[doc(hidden)]` defaulted `IntoArgs::extend_args_split(self, args, commands)` (approved additive trait change). Generated code keeps fields in declaration order, so positional numbering is unchanged, but sends subcommand args to `commands` and appends them last, including from flattened structs and enum struct variants. An earlier field-reordering approach was dropped because it swapped positionals across a `flatten`. Add tests for a field after a flattened subcommand, positionals across a `flatten`, and nested subcommand/flatten fields in a variant.
4. **Examples.** Move the `[server]`/`[database]` keys in `examples/config.toml` and `config.json` to the top level to match the flattened fields.
5. Run the AGENTS.md checks, get a fresh review, and open the PR.
