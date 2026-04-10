# COC Eval Framework Research Brief

## Purpose

Design an industrial-grade evaluation framework that measures whether COC (Cognitive Orchestration for Codegen) artifacts improve AI coding outcomes. This is a novel research question — existing benchmarks measure raw model capability, not framework-guided capability.

## Research Question

> Does encoding institutional knowledge (rules, agents, skills) into the AI's operating environment produce measurably better code than the same model operating bare?

This is distinct from:
- "Is model X better than model Y?" (standard benchmarks)
- "Can the AI follow instructions?" (instruction-following evals)
- "Is the code correct?" (functional correctness evals)

Our question is closer to: "Does the development environment matter?" — analogous to measuring whether linting, CI, and code review improve code quality vs cowboy coding.

## What We Need to Measure

### Dimension 1: Functional correctness (baseline)
Does the code work? Pass tests? Handle edge cases?
- Measured WITH and WITHOUT COC artifacts
- Same model, same problems, different environments

### Dimension 2: Rule adherence under pressure
Does the model follow project conventions when asked to violate them?
- Security rules (no hardcoded secrets, input validation)
- Architecture rules (framework-first, no raw SQL)
- Process rules (PR required, security review before commit)
- Already partially covered by 100-point governance bench

### Dimension 3: Institutional knowledge application
Does the model apply domain-specific patterns from skills/agents?
- Uses specialist agents for domain tasks
- Applies framework patterns from skill files
- Follows project-specific conventions from rules

### Dimension 4: Multi-turn coherence
Does the model maintain quality across a complex multi-step task?
- Doesn't regress on earlier fixes when making later changes
- Maintains architecture decisions across files
- Follows the plan it committed to

## Constraints

- Must run against Claude Code CLI (`claude --print`)
- Must support A/B comparison (COC vs bare, same model)
- Must support cross-model comparison (same COC, different models)
- Scoring must be execution-based where possible (tests pass/fail, not regex)
- Minimum 50 problems for statistical significance
- Must be reproducible (deterministic scaffolds, seeded where possible)

## Deliverables

1. `01-landscape.md` — Survey of existing harnesses (SWE-bench, HumanEval, etc.)
2. `02-gap-analysis.md` — What existing harnesses measure vs what we need
3. `03-framework-design.md` — COC eval framework specification
4. `04-adoption-plan.md` — Which existing benchmarks to adopt, what to build custom

## Publication Target

If the framework produces credible results, this becomes a Terrene Foundation publication:
"Measuring the Impact of Institutional Knowledge on AI Code Generation"

Filed under: `terrene/publications/` (CC BY 4.0)
