# Dependabot for Rust Only

## Intent

Enable weekly automated dependency updates for this Rust workspace while keeping Dependabot focused on Rust dependencies, and run CI for PRs targeting the default branch.

## Goals and non-goals

- Configure Dependabot to update Cargo dependencies from the workspace manifests weekly.
- Do not configure GitHub Actions or other ecosystems.
- Run the existing CI workflow for pull requests targeting both `main` and `develop`.
- Do not change dependency versions or application behavior.

## Constraints

- Use the standard `.github/dependabot.yml` format.
- The workspace has a root Cargo manifest and a derive-crate manifest.
- Preserve the existing CI behavior for `develop` while also enabling it for `main`.
- Tracked by [issue #5](https://github.com/alekece/clap-config-fallback/issues/5).

## Approaches

- **Cargo only, weekly (chosen):** one Cargo entry covers the workspace manifests, with a weekly schedule. This meets the Rust-only scope and enables timely updates.
- Cargo only, monthly: same scope with fewer update PRs, but delays dependency updates.
- Cargo plus GitHub Actions: also updates workflow dependencies, but exceeds the requested Rust-only scope.

For PR CI:

- **Add `main` to the existing `pull_request.branches` filter (chosen):** keeps `develop` behavior and runs CI for default-branch PRs.
- Retarget Dependabot PRs to `develop`: leaves default-branch dependency updates without PR coverage and does not match the requested outcome.

## Implementation

Add a single Dependabot configuration with the Cargo ecosystem and weekly schedule. Leave package selection, grouping, and update limits at Dependabot defaults. Add `main` to the existing CI workflow's PR branch filter while retaining `develop`.
