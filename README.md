# clap-config-fallback

[![Crates.io](https://img.shields.io/crates/v/clap-config-fallback.svg)](https://crates.io/crates/clap-config-fallback)
[![Docs.rs](https://docs.rs/clap-config-fallback/badge.svg)](https://docs.rs/clap-config-fallback)
[![License](https://img.shields.io/crates/l/clap-config-fallback.svg)](#license)

Add configuration-file fallback to [`clap`](https://crates.io/crates/clap) while keeping clap as the
final parser, validator, and error reporter.

`clap-config-fallback` lets one struct describe both your command line and your config file. Values
are merged, converted back into clap-compatible arguments, and parsed by clap one final time.

## Why this crate?

Merging CLI arguments with a config file is easy to get subtly wrong. Typical approaches require you
to duplicate structs, reimplement clap parsing rules, or validate config values in a separate pass.

This crate keeps **clap as the single source of truth**:

- CLI parsing, validation, conflicts, requirements, env vars, defaults, and diagnostics still come
  from clap.
- Config values are fallback values, not a second parsing system.
- The same derive shape is used for CLI and configuration.

## Installation

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
clap-config-fallback = { version = "0.2", features = ["derive"] }
```

Configuration formats are feature-gated. `toml`, `yaml`, and `json` are enabled by default. You
don't need to add `serde` yourself.

## How to use it

1. Model your CLI as a named root struct.
2. Add one `#[config(path)]` field to that root struct.
3. Call `parse_with_config()` instead of `parse()`.

The root must be a struct because the config path has to be discovered before any subcommand is
selected. Subcommands still work normally; put them behind a `#[command(subcommand)]` field. The
subcommand is always selected on the CLI; config only supplies its parameters.

```rust
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_config_fallback::{ConfigParser, ConfigSubcommand};

#[derive(Parser, ConfigParser)]
struct Cli {
    #[arg(long)]
    #[config(path)]
    config_path: Option<PathBuf>,

    #[arg(long)]
    host: String,

    #[arg(long)]
    port: u16,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, ConfigSubcommand)]
enum Command {
    Serve {
        #[arg(long)]
        workers: u16,
    },
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

[command.serve]
workers = 4
```

Running `app serve` then uses `workers = 4` unless `--workers` is given.

## How fallback works

`#[derive(ConfigParser)]` generates intermediate optional types and performs this flow:

1. Parse CLI arguments into an optional representation.
2. Resolve the config path from the `#[config(path)]` field.
3. If a path is present, load and deserialize the config file.
4. Merge values using the configured precedence.
5. Rebuild synthetic CLI arguments from the merged values.
6. Run your original clap parser for final parsing and validation.

Default precedence is:

```text
CLI > Env > Config > Default
```

If no config path is available, parsing behaves like normal clap parsing. Missing files, unsupported
formats, and disabled format features are reported as clap errors.

## Derives

Use the config derive that matches the clap derive:

| clap derive  | config derive      | Use for                      |
| ------------ | ------------------ | ---------------------------- |
| `Parser`     | `ConfigParser`     | Root CLI type                |
| `Args`       | `ConfigArgs`       | Nested or flattened arg sets |
| `Subcommand` | `ConfigSubcommand` | Subcommand enums             |

Each derive also generates two public types next to yours, `<Name>Opts` and `<Name>Config`. If one
of these names is already taken, rename them with
`#[config(opts = "...", config = "...")]` (see below).

## Requirements

- The root type must be a **named struct**, not an enum.
- The root struct should contain the `#[config(path)]` field and any `#[command(subcommand)]` field.
- Fields that participate in fallback must be serializable/deserializable and representable as CLI
  argument values.
- Nested flattened structs must derive `ConfigArgs`.
- `#[command(flatten)]` is flattened in config too, unless you add `#[config(no_flatten)]`.
- Subcommands are selected on the CLI only; config holds one optional section per subcommand.
- Type- and variant-level `#[command(...)]` attributes such as `rename_all` and `name` are honored.
  Subcommand sections follow the renamed CLI names; field keys keep the field name.
- Config types must be `'static` (the derives always produce `'static` types).

## Configuration shape

Configuration keys mirror clap's structure. Multi-word keys and subcommand sections may be written
in snake_case or kebab-case (`max_connections` or `max-connections`), but not both at once.

### Flattened args

A `#[command(flatten)]` field is flattened in config:

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    database: DatabaseArgs,
}
```

```toml
host = "localhost"
port = 5432
```

Add `#[config(no_flatten)]` to keep a nested section instead:

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    #[config(no_flatten)]
    database: DatabaseArgs,
}
```

```toml
[database]
host = "localhost"
port = 5432
```

### Subcommands

The CLI alone selects the subcommand. Config holds one optional section per subcommand, so it can
define parameters for every command at once; only the section of the command given on the CLI is
used. Nested subcommands use a nested `command` section:

```toml
[command.build]
target = "x86_64-unknown-linux-gnu"

[command.build.command.server]  # `app build server`
port = 8080

[command.run]
logging = true
```

Section names are the command names used on the CLI (`DryRun` is `[command.dry-run]`, or
`[command.dry_run]`). A variant renamed with `#[command(name = "...")]` uses that name as its
section.
Running `app build server` uses the `build` and `server` sections, `app run` uses the `run` section,
and `app` alone fails with clap's usual missing-subcommand error. Unit variants have no parameters
and need no section.

The whole file is still validated: a section for an unknown or `#[config(skip)]` subcommand, or with
invalid values, is a configuration error even when another command is selected.

### Migrating from 0.1

Config no longer selects the subcommand:

- Remove `command = "serve"` and `command = { debug = { ... } }`; pass the command on the CLI.
- Remove `#[config(tag = "...")]` and move parameters from `[command]` into `[command.<variant>]`.
- `[command.<variant>]` sections keep working, and several of them may now coexist.

## Attribute reference

Attributes are split by where they are applied. Some attributes can be used in more than one place;
those appear in both tables.

### Field and variant attributes

| Attribute                           | Use on                             | Purpose                                                  |
| ----------------------------------- | ---------------------------------- | -------------------------------------------------------- |
| `#[config(path)]`                   | root `ConfigParser` field          | Marks the config file path field                         |
| `#[config(format = "...")]`         | `#[config(path)]` field            | Forces `toml`, `yaml`, `json`, or `auto` format handling |
| `#[config(precedence = "...")]`     | field                              | Overrides fallback precedence for one field              |
| `#[config(skip)]`                   | field or subcommand variant        | Excludes one item from config fallback                   |
| `#[config(value_format = ...)]`     | field                              | Converts merged values into CLI-compatible strings       |
| `#[config(no_flatten)]`             | `#[command(flatten)]` field        | Keeps a flattened clap field as a nested config section  |
| `#[config(alias = "...")]`          | structured `#[command(...)]` field | Adds one config-only alias                               |
| `#[config(aliases = ["...", ...])]` | structured `#[command(...)]` field | Adds multiple config-only aliases                        |

### Type-level attributes

| Attribute                       | Use on                           | Purpose                                          |
| ------------------------------- | -------------------------------- | ------------------------------------------------ |
| `#[config(precedence = "...")]` | `ConfigParser`/`ConfigArgs` type | Sets fallback precedence for fields in that type |
| `#[config(skip_all)]`           | struct or subcommand enum        | Excludes all contained fields or variants        |
| `#[config(opts = "...")]`       | any config derive                | Renames the generated `<Name>Opts` type          |
| `#[config(config = "...")]`     | any config derive                | Renames the generated `<Name>Config` type        |

### `#[config(path)]`

Marks the config file path field. Supported field types are `String`, `Option<String>`, `PathBuf`,
and `Option<PathBuf>`.

If no path field exists, fallback is disabled. If the path field has a clap `default_value`, that
default path enables fallback automatically.

### `#[config(format = "...")]`

Forces parsing for the config path field:

- `toml`
- `yaml`
- `json`
- `auto` — infer from the file extension; this is the default

### `#[config(precedence = "...")]`

Controls where config values are inserted into clap's fallback chain:

| Value            | Precedence                       |
| ---------------- | -------------------------------- |
| `before_env`     | CLI > **Config** > Env > Default |
| `before_default` | CLI > Env > **Config** > Default |
| `after_default`  | CLI > Env > Default > **Config** |

`before_default` is the default. Precedence can be set on a type or a field, but it is not
propagated automatically into nested types.

```rust
#[derive(Parser, ConfigParser)]
#[config(precedence = "before_env")]
struct Cli {
    #[command(flatten)]
    args: CliArgs,
}

#[derive(Args, ConfigArgs)]
#[config(precedence = "before_default")]
struct CliArgs {
    #[arg(long, env = "APP_URL")]
    url: String,

    #[arg(short, long, default_value = "80")]
    #[config(precedence = "after_default")]
    port: u16,
}
```

### `#[config(skip)]` and `#[config(skip_all)]`

Use `skip` to exclude one field or subcommand variant from config fallback while preserving normal
clap behavior:

```rust
#[arg(long)]
#[config(skip)]
port: u16,
```

Use `skip_all` to exclude all fields or variants in the current type:

```rust
#[derive(Parser, ConfigParser)]
#[config(skip_all)]
struct Cli {
    // ...
}
```

### `#[config(value_format = ...)]`

Customizes how a merged value is converted back into a CLI argument value before the final clap
parse. This is useful for types whose config representation differs from their CLI representation.

```rust
#[arg(long)]
#[config(value_format = |value: Duration| format!("{}s", value.as_secs()))]
duration: Duration,
```

### `#[config(alias = "...")]` and `#[config(aliases = [...])]`

Adds config-only aliases for structured `#[command(...)]` fields. Aliases affect config
deserialization only; they do not change CLI flags or subcommand names.

Aliases are meaningful for subcommand fields and for flattened fields marked with
`#[config(no_flatten)]`. They are not allowed on fields that are flattened in config.

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(subcommand)]
    #[config(alias = "cmd")]
    command: Command,
}
```

### `#[config(no_flatten)]`

Prevents a `#[command(flatten)]` field from being flattened in configuration.

```rust
#[derive(Parser, ConfigParser)]
struct Cli {
    #[command(flatten)]
    #[config(no_flatten)]
    database: DatabaseArgs,
}
```

Resulting configuration:

```toml
[database]
host = "localhost"
port = 5432
```

### `#[config(opts = "...")]` and `#[config(config = "...")]`

Rename the generated types, for example when you already have a type called `CliConfig`. Either key
may be used alone; the other type keeps its default name.

```rust
#[derive(Parser, ConfigParser)]
#[config(opts = "CliOptions", config = "CliFile")]
struct Cli {
    #[arg(long)]
    port: u16,
}

struct CliConfig; // your own type, no longer clashes
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT)
at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
