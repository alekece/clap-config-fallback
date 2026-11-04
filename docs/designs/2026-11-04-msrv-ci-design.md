# MSRV Verification in CI

## Intent

CI runs only on the toolchain pinned in `rust-toolchain.toml`, so the `rust-version` declared in `Cargo.toml` is never verified. A dependency requiring a newer compiler (as `noyalib` did) goes unnoticed. Verify the declared MSRV in CI.

## Goals and non-goals

- Fail CI when the workspace does not build on the declared `rust-version`.
- Derive the version from `Cargo.toml`; bumping `rust-version` needs no CI edit.
- Check all workspace targets and features with locked dependencies.
- Do not run tests on the MSRV (decided with the maintainer: check only).
- Do not change existing jobs, dependencies, or the declared MSRV.

## Constraints

- `rust-toolchain.toml` pins 1.94.1; the job must not silently fall back to it.
- Tracked by [issue #12](https://github.com/alekece/clap-config-fallback/issues/12).

## Approaches

- **`cargo msrv verify` (chosen):** reads `rust-version` itself, installs that toolchain, and runs a custom check command. No version duplication and no hand-written script; adds one tool to install.
- Read `rust-version` with `cargo metadata` and `jq`, then install that toolchain: no extra tool, but a small script to maintain.
- Hardcode `1.89.0` in the workflow: simplest, but drifts when `rust-version` is bumped.

## Implementation

Add an `msrv` job to `.github/workflows/ci.yaml` on Linux. It installs `cargo-msrv` and runs `cargo msrv verify -- cargo check --workspace --all-targets --all-features --locked`. The `build` job waits on it alongside `lint` and `deny`. Pin or install the tool via the method the builder judges most reliable (left to the implementer); confirm the job fails on an intentionally lowered `rust-version` before delivery.
