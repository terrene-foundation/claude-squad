---
type: DECISION
date: 2026-04-08
created_at: 2026-04-08T01:55:00+08:00
author: human
session_id: unknown
session_turn: 60
project: claude-squad
topic: Strip Kailash SDK rules / hooks / context bloat from claude-squad's .claude/
phase: implement
tags: [co-artifacts, context-bloat, cleanup, hooks]
---

## Decision

claude-squad's `.claude/rules/` and `scripts/hooks/user-prompt-rules-reminder.js`
are aggressively pruned of Kailash SDK content. This repo manages OAuth
credentials and CC session rotation. It is NOT a Kailash framework consumer.

## Rationale

User reported context-bloat: per-tool-call `<system-reminder>` blocks were
dumping 150–400 lines each from rule files like `connection-pool.md`,
`patterns.md`, `python-environment.md`, `env-models.md`, `testing.md`. None
of these apply to csq — csq has no DataFlow, no Nexus, no Kaizen, no
3-tier testing model, no LLM model name routing, no `uv` virtual envs.

The session-start hook also loads ~20 rule files from terrene/.claude/rules/
plus the entire claude-squad rules directory before any actual work begins.
That's tens of thousands of tokens of irrelevant context per session.

## What Was Removed

17 rule files deleted (3230 lines): `agent-reasoning.md`, `artifact-flow.md`,
`connection-pool.md`, `cross-sdk-inspection.md`, `dataflow-pool.md`,
`deployment.md`, `documentation.md`, `e2e-god-mode.md`, `eatp.md`,
`env-models.md`, `framework-first.md`, `infrastructure-sql.md`,
`pact-governance.md`, `patterns.md`, `python-environment.md`, `testing.md`,
`trust-plane-security.md`.

3 rule files rewritten for csq scope: `agents.md` (no more dataflow/nexus/
kaizen/pact specialist references), `security.md` (now talks about OAuth /
keychain / atomic file writes — what csq actually does), `zero-tolerance.md`
(no more `terrene-foundation/kailash-py` references).

`scripts/hooks/user-prompt-rules-reminder.js`: 175 → 87 lines. Removed LLM
env discovery, "shadow agent" text, E2E/god-mode reminders, workspace
scanning. Now injects one zero-tolerance line and an optional session-notes
pointer.

13 rule files remain (down from 30): `agents.md`, `autonomous-execution.md`,
`branch-protection.md`, `cc-artifacts.md`, `communication.md`, `git.md`,
`independence.md`, `journal.md`, `learned-instincts.md`, `no-stubs.md`,
`security.md`, `terrene-naming.md`, `zero-tolerance.md`.

## What Was NOT Touched (Future Work)

- `.claude/skills/` (3.6 MB, mostly Kailash) — only loaded when invoked by
  name, not auto-loaded. Lower priority but still bloat.
- `.claude/agents/` (348 KB) — same, loaded on invoke.
- `.claude/commands/` — many Kailash workflow commands. The user uses some
  (`/start`, `/wrapup`, `/journal`, etc.); needs case-by-case review.
- `.claude/guides/` (520 KB) — content review pending.
- `scripts/hooks/validate-workflow.js` (1060 lines, mostly Kailash checks).
- Parent CLAUDE.md hierarchy (`~/repos/CLAUDE.md`, `~/repos/terrene/CLAUDE.md`)
  — out of scope for this repo, owned by terrene/.

## Consequences

- Per-turn auto-loaded rule context drops by ~75%.
- Some future tasks may need agents that no longer have rules. If a Kailash
  rule turns out to be needed for a specific edit, the user can re-add it
  scoped narrowly with `paths:` frontmatter.
- The CO sync flow from upstream (atelier/loom) will try to re-add the
  deleted files. We need a `.coc-sync-ignore` or equivalent to prevent
  reintroduction.

## For Discussion

1. Should we add a `.coc-sync-ignore` file to prevent the upstream sync
   from re-adding the deleted rules? Without it, the next sync from loom/
   would silently restore everything.
2. The 13 remaining rules average ~92 lines. Are any still too verbose?
   `cc-artifacts.md` at 256 lines is the largest — but it's `paths:`-scoped
   to `.claude/**` and `scripts/hooks/**`, so it only fires when editing
   those files. Probably fine.
3. Should the same cleanup be applied to other contrib/ repos that aren't
   Kailash SDK consumers? E.g., does `arbor/`, `astra/`, `pact/` have the
   same bloat? Out of scope for this session but worth surveying.
