# clap-config-fallback

Add config file fallback to clap **without losing clap parsing, validation, or error handling**.

---

## Why?

`clap` is excellent for parsing CLI arguments and producing high-quality error messages.

However, it does not natively support loading values from a configuration file as a fallback.

Existing solutions usually:
- reimplement parsing logic and lose clap’s behavior
- require duplicating structs (one for CLI, one for config)
- force manual validation after parsing

This crate takes a different approach:
**clap remains the single source of truth**

---

## How it works

The `ConfigParser` derive macro:

1. Generates an intermediate `Opts` struct where all fields are optional
2. Parses CLI arguments into this struct
3. Loads a configuration file (if provided)
4. Merges CLI + config (CLI has priority)
5. Reconstructs CLI arguments
6. Calls clap again for final parsing

This ensures:
- full clap validation
- consistent error messages
- no duplicated logic
- identical CLI behavior

## Requirements
Using `ConfigParser` introduces some requirements :
- Fields must:
  - implement `Serialize` / `Deserialize`
  - be convertible to CLI arguments (`ToString`-like behavior)
- Nested structs must also derive `ConfigParser`
- Only named structs are supported

## Known limitations

- `#[command(subcommand)]` are not supported yet
- Advanced `clap` features may not be fully covered
- Only file-based configuration is supported

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

Marks the field containing the configuration file path.

- optional (you can omit it entirely)
- must be:
  - `String`
  - `Option<String>`

If no `path` field is present:
**no config fallback is applied**

### `#[config(format(...))]`

Defines how the config file should be parsed.

- `toml`
- `yaml`
- `json`
- `auto`

### `#[config(skip)]`

Exclude a field from configuration fallback.

``` rust
#[arg(long)]
#[config(skip)]
port: u16,
```

### `#[config(skip_all)]`

Disable configuration fallback for all fields in the struct.

``` rust
#[config(skip_all)]
struct Cli { ... }
```
