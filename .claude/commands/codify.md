---
name: codify
description: "Load phase 05 (codify) for the current workspace. Update existing agents and skills with new knowledge."
---

## Workspace Resolution

1. If `$ARGUMENTS` specifies a project name, use `workspaces/$ARGUMENTS/`
2. Otherwise, use the most recently modified directory under `workspaces/` (excluding `instructions/`)
3. If no workspace exists, ask the user to create one first
4. Read all files in `workspaces/<project>/briefs/` for user context (this is the user's input surface)

## Phase Check

- Read `workspaces/<project>/04-validate/` to confirm validation passed
- Read `journal/` for prior decisions and discoveries
- Output: update existing agents and skills in `.claude/agents/` and `.claude/skills/`

## Execution Model

This phase executes under the **autonomous execution model** (see `rules/autonomous-execution.md`). Knowledge extraction and codification are autonomous — agents extract, structure, and validate knowledge without human intervention. The human reviews the codified output at the end (structural gate on what becomes institutional knowledge), but the extraction and synthesis process is fully autonomous.

## Workflow

### 1. Consume L5 learning artifacts

Before extracting new knowledge, integrate what the learning system has already discovered:

1. Read `.claude/learning/evolved/skills/` — if evolved skills exist, integrate them into canonical skill directories
2. Read `.claude/learning/instincts/personal/` — review high-confidence instincts for patterns worth codifying
3. Read `.claude/rules/learned-instincts.md` — verify rendered instincts are accurate and actionable

This closes the L5 feedback loop: observe → instinct → evolve → **codify**.

### 2. Deep knowledge extraction

Using as many subagents as required, peruse the project journal, CLAUDE.md, and recent session work.

- Read beyond the surface into the intent: what failure modes were discovered, what races were closed, what invariants now hold
- Understand the roles: **agents** carry procedural knowledge, **skills** distill situational knowledge, **journal** records the trail

### 3. Update existing agents

Improve agents in `.claude/agents/`. Identify which existing agent(s) should absorb the new knowledge; create a new agent only if no existing one covers the domain.

### 4. Update existing skills

Improve skills in `.claude/skills/`. Update each directory's `SKILL.md` entry point to reference new files. Skills must be detailed enough for agents to achieve situational awareness from them alone.

### 5. Update README.md and documentation (MANDATORY)

Ensure user-facing documentation reflects new capabilities. Verify README.md, docstrings, and docs build.

### 6. Red team the agents and skills

Validate that generated agents and skills are correct, complete, and secure. **claude-code-architect** verifies cc-artifacts compliance (descriptions under 120 chars, agents under 400 lines, commands under 150 lines, rules path-scoped, SKILL.md progressive disclosure).

claude-squad is a standalone downstream repo — it does not propose artifacts upstream. Artifact changes stay local.

## Agent Teams

Deploy these agents as a team for codification:

**Knowledge extraction team:**

- **deep-analyst** — Identify core patterns, architectural decisions, and domain knowledge worth capturing
- **requirements-analyst** — Distill requirements into reusable agent instructions

**Creation + validation team:**

- **intermediate-reviewer** — Review agent/skill quality before finalizing
- **claude-code-architect** — Verify cc-artifacts compliance: descriptions <120 chars, agents <400 lines, commands <150 lines, rules have `paths:` frontmatter, SKILL.md progressive disclosure, no CLAUDE.md duplication
- **gold-standards-validator** — Terrene naming, licensing accuracy, terminology standards
- **security-reviewer** — Audit agents/skills for prompt injection, insecure patterns, secrets exposure

### Journal

Create journal entries for knowledge captured:

- **DECISION** entries for what was codified and why
- **CONNECTION** entries for patterns that connect across the project
- **TRADE-OFF** entries for trade-offs in knowledge representation choices

Use sequential naming: check the highest existing `NNNN-` prefix and increment.
