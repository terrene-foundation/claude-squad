# COC Eval Framework Design

## Date: 2026-04-10
## Status: Draft — pending red team validation

## Name

**COC-Bench**: Measuring the Impact of Institutional Knowledge on AI Code Generation

## Core Design Principle

**Test what the code does, not what the model says.**

Every task has a test suite. The model's code either passes or fails. No regex scoring, no LLM-as-judge for primary metrics. Qualitative signals (rule citations, agent delegation) are secondary metadata, never part of the score.

## Experimental Design

```
                    ┌─────────────┐
                    │  Task Pool  │
                    │  (N tasks)  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌───────────┐ ┌────────┐ ┌──────────┐
        │ COC Pass  │ │  Bare  │ │ Ablation │
        │ (full     │ │ (no    │ │ (strip   │
        │ artifacts)│ │ rules, │ │ one COC  │
        │           │ │ agents,│ │ layer at │
        │           │ │ skills)│ │ a time)  │
        └─────┬─────┘ └───┬────┘ └────┬─────┘
              │            │           │
              ▼            ▼           ▼
        ┌─────────────────────────────────────┐
        │        Execution-Based Scoring       │
        │  (test suite pass/fail per task)     │
        └─────────────────────────────────────┘
              │            │           │
              ▼            ▼           ▼
        ┌─────────────────────────────────────┐
        │     Resolution Rate (pass@1)         │
        │     + Confidence Intervals           │
        │     + Delta (COC - Bare)             │
        └─────────────────────────────────────┘
```

### Independent Variable
- COC configuration: full, bare, ablation (no-rules, no-agents, no-skills, rules-only)

### Dependent Variable
- Resolution rate: % of tasks where all tests pass (binary per task)

### Controls
- Same model (e.g., Claude Opus 4.6)
- Same task set
- Same timeout
- Same max_turns
- Environment reset between tasks (git clean + checkout)
- 3-5 runs per condition for variance estimation

## Task Tiers

### Tier 1: Adopted Tasks — Functional Correctness (50-100 tasks)

Drawn from SWE-bench Verified and BigCodeBench. These test whether COC helps or hurts on standard coding tasks. COC artifacts add context (~37k tokens) — does this improve or degrade functional correctness?

**Source**: SWE-bench Verified easy+medium subset (Python)
**Scoring**: Binary — FAIL_TO_PASS tests pass, PASS_TO_PASS tests hold
**Expected COC effect**: Neutral to small positive (COC shouldn't hurt correctness, may help on tasks where patterns/agents are relevant)

### Tier 2: Security Tasks (20-30 tasks)

Custom tasks inspired by CyberSecEval. Each scaffold contains code with a security vulnerability that COC rules specifically address. The model must find and fix the vulnerability, AND the fix must pass a test suite.

**Task types**:
- Hardcoded secrets (COC rule: no hardcoded secrets)
- SQL injection (COC rule: parameterized queries)
- Command injection (COC rule: no shell=True)
- Path traversal (COC rule: validate paths)
- Insecure comparison (COC rule: constant-time compare)
- Missing input validation (COC rule: validate at boundary)

**Scoring**: Binary — vulnerability test fails before fix, passes after fix, no regressions
**Expected COC effect**: Positive — security rules should guide the model to find/fix vulnerabilities it might otherwise miss

### Tier 3: Convention Adherence (20-30 tasks)

Custom tasks that test whether the model follows project-specific conventions encoded in COC rules. Each scaffold violates a convention; the model must identify and fix the violation.

**Task types**:
- Wrong naming convention (OCEAN vs Terrene)
- Missing PR description fields
- Direct push to main (should create branch + PR)
- Missing security review before credential commit
- Stub/TODO left in production code
- Missing error handling (bare except: pass)

**Scoring**: Binary — convention test fails before fix, passes after fix
**Expected COC effect**: Strong positive — bare model has no knowledge of project conventions

### Tier 4: Architecture Coherence (10-20 tasks)

Multi-file tasks where the model must implement a feature that spans multiple files while maintaining architectural consistency. COC agents and skills provide the architecture guidance.

**Task types**:
- Add a Tauri command (must follow command pattern from rust-desktop-patterns rule)
- Add a Svelte component (must use runes pattern from svelte-patterns rule)
- Add a database migration (must use framework ORM, not raw SQL)
- Implement error handling (must use thiserror at boundary, anyhow internal)

**Scoring**: Binary — integration tests verify the feature works AND architecture tests verify patterns are followed
**Expected COC effect**: Strong positive — architecture patterns live in skills that bare mode doesn't have

## Metrics

### Primary
- **Resolution rate** (pass@1): % of tasks where all tests pass
- **Delta**: COC resolution rate minus bare resolution rate, with 95% CI
- **Per-tier delta**: Same, broken down by tier

### Secondary
- **Ablation deltas**: Which COC layer contributes most? (rules vs agents vs skills)
- **Time to resolution**: Does COC make the model faster or slower?
- **Token usage**: Does COC's context overhead increase cost?

### NOT metrics (removed)
- ~~COC bonus points~~ (biased — bare can't earn them)
- ~~Regex pattern match scores~~ (fragile, gameable)
- ~~Points out of N~~ (continuous scores obscure the binary reality of "does it work")

## Implementation Architecture

### Runner (exists: coc-eval/runner.py)

Already supports:
- COC, bare, and ablation config builds
- Environment reset between tasks
- Artifact capture (git diff, new files)
- Retry on transient failures
- ANTHROPIC_* env sanitization

Needs:
- Task loader for SWE-bench format (instance_id, repo, base_commit, test_patch)
- Execution-based scorer (run test suite, check exit code)
- Confidence interval calculation
- JSON output with per-task binary results

### Task Format

```python
{
    "id": "COC-SEC-001",
    "tier": "security",
    "source": "custom",  # or "swe-bench-verified", "bigcodebench"
    "scaffold": "sec-001-hardcoded-secret",
    "scaffold_files": ["api_client.py", "tests/test_api_client.py"],
    "prompt": "Review api_client.py for security issues. Fix any you find.",
    "test_command": "python -m pytest tests/test_api_client.py -v",
    "pass_criteria": "exit_code == 0",
    "max_turns": 10,
    "timeout": 300,
}
```

### Scoring

```python
def score_task(task, coc_env_path):
    """Run test suite, return binary pass/fail."""
    result = subprocess.run(
        task["test_command"].split(),
        cwd=coc_env_path,
        capture_output=True,
        timeout=60,
    )
    return result.returncode == 0
```

No regex. No LLM judge. Tests pass or they don't.

## Phased Rollout

| Phase | Tasks | Scoring | Timeline |
|---|---|---|---|
| Phase 1 | 50 tasks (20 SWE-bench + 20 security + 10 convention) | Execution-based | 1 session |
| Phase 2 | 100 tasks (+ BigCodeBench + architecture) | Execution-based | 1 session |
| Phase 3 | 150+ tasks, 3-5 runs, confidence intervals | Execution + secondary metrics | 2 sessions |
| Publication | Results with full methodology | Peer-reviewable | After Phase 3 |

## Publication Outline

**Title**: "Measuring the Impact of Institutional Knowledge on AI Code Generation"

1. **Introduction**: The vibe coding problem — AI models code without conventions
2. **Related Work**: SWE-bench, BigCodeBench, CyberSecEval (none test framework-guided coding)
3. **COC Framework**: Brief description of the 5-layer architecture
4. **Experimental Design**: A/B + ablation, execution-based scoring, statistical methodology
5. **Results**: Per-tier resolution rates, deltas, confidence intervals, ablation analysis
6. **Discussion**: Which COC layers matter most? When does COC hurt? Cost/benefit tradeoff
7. **Threats to Validity**: Contamination, task selection bias, single-model limitations
8. **Conclusion**: Institutional knowledge measurably improves/doesn't improve AI coding

**Venue**: Terrene Foundation publication (CC BY 4.0), cross-posted to arXiv
