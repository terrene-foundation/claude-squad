---
type: DECISION
date: 2026-04-10
created_at: 2026-04-10T18:05:00+08:00
author: agent
session_id: v2-scaffold
session_turn: 50
project: csq-v2
topic: Three-crate workspace coexisting with v1.x Python/bash
phase: implement
tags: [architecture, cargo, workspace]
---

# Three-Crate Workspace Coexisting with v1.x

## Decision

The Rust workspace lives alongside v1.x files at the repo root:

- `csq-core/` — library crate with all business logic, zero Tauri dependency
- `csq-cli/` — binary crate producing `csq` (via `target/debug/csq`)
- `src-tauri/` — Tauri binary (to be added in M0-02)

v1.x `csq` (bash script), `rotation-engine.py`, `dashboard/`, etc. remain untouched. The Rust binary is at `target/debug/csq`, not conflicting with `./csq`.

## Rationale

Coexistence allows incremental migration. Users run v1.x until v2.0 reaches CLI parity. The v1.x install path (`~/.local/bin/csq`) is only replaced when `csq install` runs the v2.0 binary.

## Consequences

- `Cargo.lock` is gitignored (library crates don't commit lockfiles; we'll add it when the CLI is distributed)
- `.gitignore` updated with `target/`, `Cargo.lock`, and credential paths
- CI must handle both Python tests and Rust tests

## For Discussion

- Should `Cargo.lock` be committed once v2.0 reaches alpha release (reproducible builds)?
- When v1.x is fully deprecated, should the Python files be moved to a `v1/` archive directory or deleted entirely?
