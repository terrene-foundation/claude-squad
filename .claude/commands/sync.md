---
description: "Sync disabled — claude-squad has forked from the upstream USE template"
---

claude-squad was originally scaffolded from the `kailash-coc-claude-py` USE template, but the `.claude/` artifact set has been pruned and adapted to fit this project (a small Python/bash OAuth rotation tool). The Kailash SDK skills, framework agents, frontend agents, and SDK commands were removed because they are irrelevant to csq's actual work and burn context on every turn.

**This command is a deliberate no-op.**

Running `/sync` would pull the Kailash USE template back in and undo the cleanup. If you genuinely need an artifact from the upstream template:

1. Identify the specific file you want (e.g., `rules/new-generic-rule.md`)
2. Copy it in manually from `../../loom/kailash-coc-claude-py/`
3. Adapt it to claude-squad's context before committing

## Report

```
Sync is disabled for claude-squad.
Template: kailash-coc-claude-py (forked)
Rationale: artifact set pruned for csq's scope — a Python/bash OAuth rotation tool
Policy: no automatic re-pulls; manual cherry-pick only
```

## When an artifact needs to flow back to csq

If you discover a generic COC improvement while working on claude-squad (e.g., a better phrasing of a rule), consider whether it belongs in loom/ as a global improvement, or in atelier/ as a CO principle. Propose upstream by opening a PR against the appropriate repo. Do NOT attempt to propagate via the template sync mechanism — it was designed for Kailash SDK users.
