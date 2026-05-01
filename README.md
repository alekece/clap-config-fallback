# clap-config-fallback

Add configuration-file fallback to clap **without losing clap parsing, validation, or error handling**.

## Why?

`clap` is excellent at parsing CLI arguments and producing high-quality diagnostics.

What it does not do out of the box is _merge a config file with CLI arguments while preserving the
same parsing and validation contract_.

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
command = "serve"
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

Using `clap-config-fallback` has a few constraints:

- The root type must be a **named struct**.
- Fields that participate in fallback must be serializable/deserializable.
- Field values must be representable as CLI argument values.
- Nested flattened structs must also derive `ConfigArgs`.
- Subcommands use an **externally tagged representation** by default.
- Subcommands must follow the **canonical structure** (see below).

## Canonical structure

`ConfigParser` **cannot be derived on enums**, which is the only intentional incompatibility with
clap.

Configuration fallback requires resolving the configuration file **before** parsing the command.

With an enum as root type, the command must be selected first, which makes it impossible to define or
access a config path beforehand.

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

## Subcommand representation

By default, subcommands use an **externally tagged representation** in configuration:

```toml
command = "serve"

command = { debug = { "verbose" = true } }

[command.build]
target = "x86_64-unknown-linux-gnu"
```

Alternatively, you can opt into an **internally tagged representation** using:

```rust
#[derive(Subcommand, ConfigSubcommand)]
#[config(tag = "name")]
enum Command { ... }

```

Resulting to the following configuration:

```toml
[command]
name = "build"
target = "x86_64-unknown-linux-gnu"
```

> ⚠️ **Note**  
> The tag field shares the same namespace as variant fields, and may conflict with them.

## Derive attributes

### `#[config(path)]`

Marks the field that stores the configuration file path.

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
- `auto` (**default**) - Detect format by extension

### `#[config(skip)]`

Excludes a field from config fallback while keeping normal CLI parsing.

```rust
#[arg(long)]
#[config(skip)]
port: u16,
```

### `#[config(skip_all)]`

Disables config fallback for all fields.

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

```rust
#[derive(Subcommand, ConfigSubcommand)]
#[config(tag = "name")]
enum Command { ... }
```

### `#[config(alias = "...", aliases = ["...", "..."])]`

Adds configuration-only aliases for fields using `#[command(...)]`.

While clap flattens these fields in the CLI, they still appear as structured keys in configuration
files. Aliases allow alternative section names to be accepted during configuration deserialization,
without changing CLI behavior.

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(subcommand)]
    #[config(alias = "cmd")]
    command: Command,
}
```

## Attribute reference

| Attribute                             | `ConfigParser` | `ConfigArgs` | `ConfigSubcommand` | Purpose                                                                |
| ------------------------------------- | :------------: | :----------: | :----------------: | ---------------------------------------------------------------------- |
| `#[config(path)]`                     |       ✅       |      ❌      |         ❌         | Marks the config file path field                                       |
| `#[config(format = "...")]`           |       ✅       |      ❌      |         ❌         | Forces config format: `toml`, `yaml`, `json`, or `auto`                |
| `#[config(skip)]`                     |       ✅       |      ✅      |         ✅         | Excludes a field or variant from config generation                     |
| `#[config(skip_all)]`                 |       ✅       |      ✅      |         ✅         | Excludes all fields or variants from config generation                 |
| `#[config(value_format = ...)]`       |       ✅       |      ✅      |         ✅         | Converts a merged value back into a CLI-compatible string              |
| `#[config(tag = "...")]`              |       ❌       |      ❌      |         ✅         | Enables internally tagged subcommand representation                    |
| `#[config(alias = "...")]`            |       ✅       |      ✅      |         ✅         | Adds one configuration-only alias for a `#[command(...)]` field        |
| `#[config(aliases = ["...", "..."])]` |       ✅       |      ✅      |         ✅         | Adds multiple configuration-only aliases for a `#[command(...)]` field |
