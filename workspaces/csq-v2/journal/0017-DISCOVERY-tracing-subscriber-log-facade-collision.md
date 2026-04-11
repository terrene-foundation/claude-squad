---
type: DISCOVERY
date: 2026-04-11
created_at: 2026-04-11T18:30:00+08:00
author: agent
session_id: session-2026-04-11c
session_turn: 190
project: csq-v2
topic: tracing-subscriber default tracing-log feature collides with tauri-plugin-log
phase: redteam
tags: [desktop, logging, tracing, tauri, dependency, ci]
---

# DISCOVERY: tracing-subscriber's default `tracing-log` feature silently installs the `log` facade

## The trap

`tracing-subscriber = { version = "0.3", features = ["env-filter"] }` — a standard-looking dep — enables the `tracing-log` default feature. Inside `try_init()` this calls `LogTracer::init()` which calls `log::set_boxed_logger(...)`.

If the same process also registers `tauri-plugin-log`, that plugin's `attach_logger` ALSO calls `log::set_boxed_logger`. Second call returns `SetLoggerError`, which `?`-propagates out of `.plugin()`, out of `.setup()`, out of `.run()`, and finally hits `.expect("error while running tauri application")` → **the app panics at startup**.

## Why CI missed it

`cargo test --workspace` and `cargo clippy --workspace --all-targets` compile the desktop lib but never invoke `run()`. The panic is a runtime-only symptom. A fully green CI reported the app as healthy when it would not even start.

## Fix

Workspace dep: `tracing-subscriber = { version = "0.3", default-features = false, features = ["fmt", "env-filter", "std", "ansi", "smallvec"] }`. Disabling defaults drops `tracing-log` entirely; `try_init()` no longer touches the `log` facade. tauri-plugin-log owns `log::*`; tracing-subscriber owns `tracing::*`. Both coexist.

## Consequences

Add a CI smoke test that actually launches the desktop app (e.g. `cargo tauri build` + run the binary with `--help` or a `CSQ_SMOKE_TEST=1` env var that exits right after `setup`). Runtime panics from plugin registration should not reach main.

## For Discussion

1. Should every workspace dep with `features = [...]` also specify `default-features = false` to force explicit opt-in? The `tracing-log` trap exists in many other default-feature crates — how many other silent side effects are we inheriting?
2. If CI had a "smoke launch" step (compile + start + kill), what other classes of runtime-only failures would it catch that `cargo test` can't?
3. Would replacing `.expect(...)` with a structured error return at `run()` let us keep the app running in degraded mode when plugin setup fails, instead of panicking?
