# Dependabot for Rust Only

## Intent

Enable weekly automated dependency updates for this Rust workspace while keeping Dependabot focused on Rust dependencies.

## Goals and non-goals

- Configure Dependabot to update Cargo dependencies from the workspace manifests weekly.
- Do not configure GitHub Actions or other ecosystems.
- Do not change dependency versions or application behavior.

## Constraints

- Use the standard `.github/dependabot.yml` format.
- The workspace has a root Cargo manifest and a derive-crate manifest.
- Tracked by [issue #5](https://github.com/alekece/clap-config-fallback/issues/5).

## Approaches

- **Cargo only, weekly (chosen):** one Cargo entry covers the workspace manifests, with a weekly schedule. This meets the Rust-only scope and enables timely updates.
- Cargo only, monthly: same scope with fewer update PRs, but delays dependency updates.
- Cargo plus GitHub Actions: also updates workflow dependencies, but exceeds the requested Rust-only scope.

## Implementation

Add a single Dependabot configuration with the Cargo ecosystem and weekly schedule. Leave package selection, grouping, and update limits at Dependabot defaults.
