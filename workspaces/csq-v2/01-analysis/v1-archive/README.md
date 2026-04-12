# v1 Archive — Frozen Historical Context

Content in this directory describes the csq v1.x bash + Python architecture.
csq v2 is a Rust rewrite that does not share code with v1. These documents
are preserved as historical context for the requirements elicitation — they
are NOT authoritative for the current Rust implementation.

For the authoritative csq v2 specs, see `/specs/` at the repo root.
For the v2 architecture decisions, see `workspaces/csq-v2/01-analysis/01-research/03-architecture-decision-records.md`.

## Files

- `05-security-analysis.md` — Security audit of v1.x Python code (rotation-engine.py, dashboard/\*.py, csq bash wrapper). Retained because the threat model it describes still applies to the v2 implementation, even though the specific code it audits is gone.
- `06-v1x-issues-registry.md` — Known issues in v1.x that v2 was designed to fix. Useful as a "don't regress" checklist.

## Status

- Frozen as of 2026-04-12 during the handle-dir model adoption (journal 0031, spec 02).
- Content here MUST NOT be updated. If v1 behavior is ever revisited, create a new entry in `journal/` linking here.
