# Gap Analysis: Existing Harnesses vs COC Eval Needs

## Date: 2026-04-10

## The Novel Question

COC asks: **"Does institutional knowledge injected into the AI's operating context improve coding outcomes?"**

This maps to a controlled experiment:
- **Treatment**: Model + COC artifacts (rules, agents, skills, commands)
- **Control**: Same model, bare (no artifacts)
- **Measurement**: Code quality across multiple dimensions

No existing harness measures this. Here's what they cover and what's missing.

## Coverage Matrix

| Dimension | SWE-bench | BigCodeBench | CyberSecEval | Aider | LiveCodeBench | COC Needs |
|---|---|---|---|---|---|---|
| Functional correctness | Full | Full | Partial | Full | Full | Baseline requirement |
| Security rule adherence | None | None | **Full** | None | None | Critical for COC |
| Architecture pattern use | None | Partial (libraries) | None | None | None | Core COC value |
| Convention adherence | None | None | None | None | None | Core COC value |
| Multi-turn coherence | Partial (agent) | None | None | Partial | None | Important for COC |
| Framework-guided vs bare | None | None | None | None | None | **THE question** |
| Cross-model comparison | Yes | Yes | Yes | Yes | Yes | Supported by all |
| Anti-contamination | SWE-bench Live | None | N/A | None | **Yes** | Important |

## Gaps

### Gap 1: No A/B framework eval exists anywhere

Every harness tests "Model A vs Model B on task X." None test "Model A with framework F vs Model A without framework F on task X." This is COC's core question and it's entirely novel.

### Gap 2: Convention adherence has no benchmark

Does the model follow project naming conventions? Use the project's preferred patterns? Apply the right abstractions? No harness measures this. COC's rules layer (Layer 3) is specifically designed to enforce conventions, but there's no way to measure whether it works.

### Gap 3: Security rule enforcement under adversarial pressure

CyberSecEval measures insecure code propensity but doesn't test whether a security framework can PREVENT the model from generating insecure code when socially engineered. Our 100-point governance bench partially covers this (adversarial rubric), but it tests knowledge, not code output.

### Gap 4: Multi-file architectural coherence

SWE-bench tests bug fixes (usually single-file). BigCodeBench tests function completion. Neither tests whether the model maintains architectural decisions across a multi-file feature implementation — the kind of work COC agents (Layer 1) coordinate.

## What We Can Adopt vs What We Must Build

| Need | Adopt From | Build Custom |
|---|---|---|
| Functional correctness baseline | SWE-bench Verified (50-100 tasks) | Adapt runner.py to SWE-bench format |
| Library usage quality | BigCodeBench (50 tasks) | Wrapper for function-level tasks |
| Security rule enforcement | CyberSecEval concepts | Custom tasks with COC security rules |
| Convention adherence | None | Full custom (20-30 tasks) |
| Architecture coherence | None | Full custom (10-20 tasks) |
| Adversarial rule resistance | Our governance bench (exists) | Expand from 10 to 20+ adversarial tests |
| A/B framework comparison | None (novel) | Extend runner.py's COC/bare/ablation modes |

## Summary

- **~50% of tasks can be adopted** from SWE-bench and BigCodeBench
- **~30% need custom design** for convention adherence and architecture coherence
- **~20% already exist** in our governance bench (adversarial tests)
- The A/B experimental design (COC vs bare) is novel infrastructure we already have in runner.py
- Scoring MUST shift from regex to execution-based for adopted tasks
