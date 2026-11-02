# Dependabot for Rust Only Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable weekly Dependabot updates for Cargo dependencies only.

**Architecture:** Add one standard Dependabot YAML configuration under `.github/`. A Cargo entry at the repository root covers the workspace and its derive crate; no other ecosystems are enabled.

**Tech Stack:** GitHub Dependabot YAML; Python 3 with PyYAML for local syntax and content assertions.

**Spec:** `docs/superpowers/specs/2026-11-02-dependabot-rust-only-design.md`

## Global Constraints

- Use the standard `.github/dependabot.yml` format.
- Configure Cargo updates weekly.
- Do not configure GitHub Actions or other ecosystems.
- Do not change dependency versions or application behavior.

## Review Focus

- Malformed YAML or an unexpected document shape — parse the file and assert its parsed structure.
- Wrong manifest location or schedule — assert the Cargo entry uses `/` and `weekly`.
- Accidental non-Rust update scope — assert the sole ecosystem is `cargo`.

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
