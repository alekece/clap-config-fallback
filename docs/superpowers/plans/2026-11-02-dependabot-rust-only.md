# Dependabot for Rust Only Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable weekly Cargo-only Dependabot updates and run CI for pull requests targeting `main` and `develop`.

**Architecture:** Configure one weekly Cargo update entry at the repository root. Extend only the CI workflow's pull-request branch filter to include `main`, retaining `develop` and existing push triggers.

**Tech Stack:** GitHub Dependabot YAML, GitHub Actions YAML, Python 3 with PyYAML, and actionlint.

**Spec:** `docs/superpowers/specs/2026-11-02-dependabot-rust-only-design.md`

## Global Constraints

- Use the standard `.github/dependabot.yml` format.
- Configure Cargo updates weekly.
- Do not configure GitHub Actions or other ecosystems in Dependabot.
- Do not change dependency versions or application behavior.
- Preserve the existing CI behavior for `develop` and push triggers.

## Review Focus

- Malformed Dependabot YAML or an unexpected document shape — Task 1 asserts the exact parsed configuration.
- Invalid workflow YAML or a missing PR branch — Task 2 runs actionlint and asserts the exact PR branch list.
- Accidental changes to push CI scope — Task 2 asserts the push branch list remains `main` and `develop`.

---

### Task 1: Configure Cargo Dependabot updates

**Files:**
- Create: `.github/dependabot.yml`

**Interfaces:**
- Consumes: the root Cargo workspace manifest and its `clap-config-fallback-derive` member.
- Produces: a Dependabot configuration containing one weekly Cargo update entry rooted at `/`.

- [x] **Step 1: Add the minimal Dependabot configuration**

Create `.github/dependabot.yml` with `version: 2` and exactly one update entry: `package-ecosystem: cargo`, `directory: "/"`, and a `schedule` with `interval: weekly`. Leave all other Dependabot settings at their defaults.

- [x] **Step 2: Validate YAML and the requested scope**

Run:

```sh
python3 -c 'import yaml; from pathlib import Path; config = yaml.safe_load(Path(".github/dependabot.yml").read_text()); assert config == {"version": 2, "updates": [{"package-ecosystem": "cargo", "directory": "/", "schedule": {"interval": "weekly"}}]}'
```

Expected: exit code 0; this verifies valid YAML, a single root Cargo entry, a weekly schedule, and no other ecosystems.

- [x] **Step 3: Commit the approved design, plan, and configuration**

```sh
git add .github/dependabot.yml docs/superpowers/specs/2026-11-02-dependabot-rust-only-design.md docs/superpowers/plans/2026-11-02-dependabot-rust-only.md
git commit -m "ci: configure Dependabot for Cargo only" -m "Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```

### Task 2: Run CI for pull requests targeting main

**Files:**
- Modify: `.github/workflows/ci.yaml`

**Interfaces:**
- Consumes: the existing `pull_request.branches` filter, which currently includes `develop`.
- Produces: a pull-request filter containing exactly `main` and `develop`, without changing the existing push filter or CI jobs.

- [x] **Step 1: Write the failing branch-filter assertion**

Run:

```sh
python3 -c 'import yaml; from pathlib import Path; workflow = yaml.load(Path(".github/workflows/ci.yaml").read_text(), Loader=yaml.BaseLoader); assert workflow["on"]["pull_request"]["branches"] == ["main", "develop"]; assert workflow["on"]["push"]["branches"] == ["main", "develop"]'
```

Expected: FAIL with `AssertionError` because the pull-request filter currently contains only `develop`.

- [x] **Step 2: Include main in the pull-request filter**

Add `main` to `.github/workflows/ci.yaml` under `on.pull_request.branches`; retain `develop` and do not change `on.push` or any job.

- [x] **Step 3: Validate workflow syntax and branch scope**

Run:

```sh
actionlint .github/workflows/ci.yaml && python3 -c 'import yaml; from pathlib import Path; workflow = yaml.load(Path(".github/workflows/ci.yaml").read_text(), Loader=yaml.BaseLoader); assert workflow["on"]["pull_request"]["branches"] == ["main", "develop"]; assert workflow["on"]["push"]["branches"] == ["main", "develop"]'
```

Expected: exit code 0; actionlint accepts the workflow, PRs to `main` and `develop` are covered, and push triggers are unchanged.

- [x] **Step 4: Commit the CI change and revised project records**

```sh
git add .github/workflows/ci.yaml docs/superpowers/specs/2026-11-02-dependabot-rust-only-design.md docs/superpowers/plans/2026-11-02-dependabot-rust-only.md
git diff --cached --check
git commit -m "ci: validate Dependabot PRs on main" -m "Co-authored-by: Copilot App <223556219+Copilot@users.noreply.github.com>"
```
