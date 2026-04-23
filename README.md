# clap-config-fallback

Add configuration-file fallback to clap **without losing clap parsing, validation, or error handling**.

---

## Why?

`clap` is excellent at parsing CLI arguments and producing high-quality diagnostics.

What it does not do out of the box is *merge a config file with CLI arguments while still enforcing  
the same clap contract*.

Common alternatives often:
- reimplement parsing and drift from clap behavior,
- duplicate structs (one for CLI, one for config), or
- validate after parsing in a separate pass.

`clap-config-fallback` keeps **clap as the single source of truth**.

---

## How it works

`#[derive(ConfigParser)]` generates an intermediate optional `Opts` struct and runs a two-phase parse:

1. Parse CLI arguments into `Opts`.
2. Resolve a config file path (if one is defined).
3. Load and deserialize config into a generated `Config` struct.
4. Merge values with precedence **CLI > config**.
5. Reconstruct synthetic CLI args from merged `Opts`.
6. Run your original clap parser for final parsing + validation.

Because the final pass is still clap, you keep clap’s errors and validation behavior.

## Precedence and edge cases

- **CLI always wins** over config for the same field.
- Config fallback only runs when a `#[config(path)]` field is present and resolves to `Some(path)`.
- If the path field has a clap `default_value`, that default also enables fallback.
- If no path is available, parsing behaves like normal clap parsing.
- A non-existent file is reported as a clap `Io` error.
- Unknown/unsupported format (or feature-disabled format) is reported as a clap `InvalidValue` error.

## Requirements

Using `ConfigParser` has a few constraints:
- The target type must be a **named struct**.
- Fields that participate in fallback must be serializable/deserializable.
- Field values must be representable as CLI argument values (`to_string()` output is used).
- Nested flattened structs must also derive `ConfigParser`.

## Known limitations

- `#[command(subcommand)]` is not supported yet.
- Some advanced clap attribute combinations may not be fully covered.
- Only file-based configuration is currently supported.

---

## Example

```rust
use clap::Parser;
use clap_config_fallback::ConfigParser;

#[derive(Debug, Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    debug: bool,

    #[arg(long)]
    profile: String,

    #[arg(long)]
    threads: u16,

    #[arg(long, default_value = "config.toml")]
    #[config(path)]
    config_path: String,
}

fn main() {
    let cli = Cli::parse_with_config();
    println!("{cli:#?}");
}
```

## `ConfigParser` attributes

### `#[config(path)]`

Marks the field that stores the configuration file path.

- Optional (you may omit it entirely).
- Must be one of:
  - `String`
  - `Option<String>`

If no `path` field exists, config fallback is disabled.

### `#[config(format = "...")]`

Forces how the config file is parsed for the `#[config(path)]` field.

Supported values:
- `toml`
- `yaml`
- `json`
- `auto` (default; detect by extension)

### `#[config(skip)]`

Excludes a field from config fallback while keeping normal CLI parsing.

```rust
#[arg(long)]
#[config(skip)]
port: u16,
```

### `#[config(skip_all)]`

Disables config fallback for all fields in the struct.

```rust
#[config(skip_all)]
struct Cli { /* ... */ }
```
