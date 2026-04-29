# clap-config-fallback
> Merge CLI arguments and config files without duplication, while preserving clap's behavior.

Add configuration-file fallback to clap **without losing clap parsing, validation, or error handling**.

## Why?

`clap` is excellent at parsing CLI arguments and producing high-quality diagnostics.

What it does not do out of the box is *merge a config file with CLI arguments while preserving the
same parsing and validation contract*.

Common alternatives often:
- reimplement parsing and drift from clap behavior,
- duplicate structs (one for CLI, one for config), or
- validate after parsing in a separate pass.

`clap-config-fallback` keeps **clap as the single source of truth for parsing and validation**.

## Installation

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
clap-config-fallback = { version = "0.1", features = ["derive"] }
```

Optional format features:

- `toml`
- `yaml`
- `json`

All three are enabled by default.

## Quick start

```rust
use clap::{Parser, Subcommand};
use clap_config_fallback::{ConfigParser, ConfigSubcommand};

#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config: Option<String>,

    #[arg(long)]
    host: String,

    #[arg(long)]
    port: u16,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, ConfigSubcommand)]
#[config(tag = "ref")]
enum Command {
    Serve,
}

fn main() {
    let cli = Cli::parse_with_config();
    println!("{}:{}", cli.host, cli.port);
}
```

Example `config.toml`:

```toml
host = "127.0.0.1"
port = 8080
command = { ref = "serve" }
```

## How it works

`#[derive(ConfigParser)]` generates an intermediate optional `Opts` struct and runs a two-phase parse:

1. Parse CLI arguments into `Opts`.
2. Resolve a config file path (if one is defined).
3. Load and deserialize config into a generated `Config` struct.
4. Merge values with precedence **CLI > config**.
5. Reconstruct synthetic CLI args from merged `Opts`.
6. Run your original clap parser for final parsing + validation.

Because the final pass is still clap, you keep clap’s errors and validation behavior.

## Derives overview

`clap-config-fallback` provides three derives that mirror clap's structure and are designed to be 
used together:

| clap         | clap-config-fallback | Role                          |
| ------------ | -------------------- | ----------------------------- |
| `Parser`     | `ConfigParser`       | Root CLI + config entry point |
| `Args`       | `ConfigArgs`         | Nested argument groups        |
| `Subcommand` | `ConfigSubcommand`   | Enum-based subcommands        |

## Precedence and edge cases

- **CLI always wins** over config for the same field.
- Config fallback only runs when a `#[config(path)]` field is present and resolves to `Some(path)`.
- If the path field has a clap `default_value`, that default also enables fallback.
- If no path is available, parsing behaves like normal clap parsing.
- A non-existent file is reported as a `clap` error.
- Unknown/unsupported format (or feature-disabled format) is reported as a `clap` error.

## Requirements

Using `ConfigParser` has a few constraints:
- The root type must be a **named struct**.
- Fields that participate in fallback must be serializable/deserializable.
- Field values must be representable as CLI argument values.
- Nested flattened structs must also derive `ConfigArgs`.
- Subcommands use an **internally tagged representation** in the configuration file.
- Subcommands must follow the **canonical structure** (see below).

## Canonical structure

Subcommands must follow this structure to enable configuration fallback.

`ConfigParser` **cannot be derived on enums**, which is the only intentional incompatibility with
clap.

Configuration fallback requires resolving the configuration file **before** parsing the command.

With an enum root, the command must be selected first, which makes it impossible to define or access
a config path beforehand.

Use a root struct that defines the config path and delegates to subcommands:

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config_path: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, ConfigSubcommand)]
#[config(tag = "ref")]
enum Command {
    Run,
    Build(BuildCommand),
    Debug {
        #[arg(long)]
        verbose: bool,
    },
}

#[derive(Args, ConfigArgs)]
struct BuildCommand {
    #[arg(long)]
    target: String,
}

```

## Derive attributes
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

### `#[config(value_format = ...)]`

Controls how a value is converted back into CLI arguments after merging.

```rust
#[arg(long)]
#[config(value_format = format!("{}s", duration.as_secs()))]
duration: Duration,
```

### `#[config(tag = "...")]`

Defines the field used to select the active subcommand in the configuration.

**Required for `ConfigSubcommand`**

## Attribute reference

| Attribute | `ConfigParser` | `ConfigArgs` | `ConfigSubcommand` | Purpose |
|---|:---:|:---:|:---:|---|
| `#[config(path)]` | ✅ | ❌ | ❌ | Marks the config file path field |
| `#[config(format = "...")]` | ✅ | ❌ | ❌ | Forces config format: `toml`, `yaml`, `json`, or `auto` |
| `#[config(skip)]` | ✅ | ✅ | ✅ | Excludes a field or variant from config generation |
| `#[config(skip_all)]` | ✅ | ✅ | ✅ | Excludes all fields or variants from config generation |
| `#[config(value_format = ...)]` | ✅ | ✅ | ✅ | Converts a merged value back into a CLI-compatible string |
| `#[config(tag = "...")]` | ❌ | ❌ | ✅ | Defines the config field used to select a subcommand variant |
