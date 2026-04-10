# Eval Harness Landscape Survey

## Date: 2026-04-10

## 1. Harness Catalog

| Harness | What It Measures | Tasks | Scoring | A/B Support | Open Source | Key Limitation |
|---|---|---|---|---|---|---|
| **SWE-bench** (Princeton) | Bug fixing: resolve real GitHub issues | 2,294 (12 Python repos) | Execution-based, binary (FAIL_TO_PASS + PASS_TO_PASS) | Yes (swap system prompt / agent framework) | [github.com/SWE-bench/SWE-bench](https://github.com/SWE-bench/SWE-bench) | Python-only; many tasks ambiguous |
| **SWE-bench Verified** | Same, human-validated subset | 500 | Same binary execution | Yes | Same repo | Near-saturated (~81%); 196 "easy" tasks inflate |
| **SWE-bench Pro** (Scale AI) | Multi-file, multi-language, long-horizon | 1,865 (41 repos, Py/Go/TS/JS) | Same binary execution | Yes | Partial (731 public) | Top ~23-78%; private set limits reproducibility |
| **SWE-bench Live** (Microsoft) | Continuously updated post-training-cutoff | Rolling | Same | Yes | [github.com/microsoft/SWE-bench-Live](https://github.com/microsoft/SWE-bench-Live) | Anti-contamination; newer |
| **SWE-bench Multilingual** | Cross-language variant | 300 (9 languages, 42 repos) | Same | Yes | Yes | Smaller |
| **HumanEval** (OpenAI) | Single-function completion | 164 | pass@k (execution) | Possible but trivial | [github.com/openai/human-eval](https://github.com/openai/human-eval) | Saturated (>95%); contaminated |
| **HumanEval+** (EvalPlus) | Same tasks, 80x more tests | 164 | pass@k expanded | Same | [github.com/evalplus/evalplus](https://github.com/evalplus/evalplus) | Still 164 problems |
| **MBPP / MBPP+** | Basic Python programming | 974 | pass@k execution | Same | Same EvalPlus repo | Trivial; saturated |
| **BigCodeBench** (ICLR'25) | Practical multi-library tasks | 1,140 (139 libraries, 7 domains) | Execution (avg 5.6 tests/task, 99% branch coverage) | Yes | [github.com/bigcode-project/bigcodebench](https://github.com/bigcode-project/bigcodebench) | Function-level only |
| **Aider Polyglot** | Code editing across 6 languages | 225 (from Exercism) | Binary test pass/fail | Model-only (no framework) | [github.com/Aider-AI/polyglot-benchmark](https://github.com/Aider-AI/polyglot-benchmark) | No agent framework A/B |
| **LiveCodeBench** | Fresh competitive programming | 1,055 (v6, May 2023-Apr 2025) | Execution pass@1 + self-repair | Yes | [github.com/LiveCodeBench/LiveCodeBench](https://github.com/LiveCodeBench/LiveCodeBench) | Algorithmic focus |
| **CyberSecEval 4** (Meta) | Insecure code propensity + security patching | ~1,000+ (insecure) + 136 (AutoPatchBench) | LLM-judged + execution (fuzzer-verified) | Yes | [github.com/meta-llama/PurpleLlama](https://github.com/meta-llama/PurpleLlama) | C/C++ focus for patching |
| **AgentBench** (Tsinghua) | Multi-turn agent reasoning | ~450 (8 environments) | Task completion rate | Yes | [github.com/THUDM/AgentBench](https://github.com/THUDM/AgentBench) | Stale (GPT-4 era); not code-specific |

## 2. Scoring Method Hierarchy

From most to least credible:

1. **Execution-based** (tests pass/fail) — SWE-bench, HumanEval, BigCodeBench. Gold standard. Binary, no ambiguity.
2. **LLM-as-judge** (model grades output against rubric) — CyberSecEval insecure code suite. Useful for qualitative aspects where tests are hard to write.
3. **Regex/pattern matching** — Our current coc-eval. Fragile, gameable, low credibility.

## 3. Statistical Significance Requirements

For binary outcome (pass/fail), 95% confidence, 80% power:

| Effect Size (bare vs COC) | Required Tasks Per Condition |
|---|---|
| 20 percentage points (e.g., 40% vs 60%) | ~100 tasks |
| 15 percentage points | ~180 tasks |
| 10 percentage points | ~400 tasks |

Our current 5 tests cannot establish statistical significance for any effect size.

## 4. Relevance Ranking for COC Evaluation

| Rank | Harness | Why Relevant | Adoption Effort |
|---|---|---|---|
| 1 | **SWE-bench Verified** | Real bug fixing, binary execution, supports A/B config swap, 500 tasks | Medium — need to adapt runner.py to SWE-bench task format |
| 2 | **BigCodeBench** | Multi-library practical tasks, tests whether COC rules improve library usage | Medium — function-level, need wrapper |
| 3 | **CyberSecEval 4** | Directly tests insecure code generation — maps to COC security rules | High — different architecture (C/C++ focus) |
| 4 | **Aider Polyglot** | Clean 225-problem set, binary scoring, 6 languages | Low — simple test-pass-fail model |
| 5 | **LiveCodeBench** | Anti-contamination, 1,055 problems, execution-based | Medium — algorithmic focus, less COC surface |

Least relevant: HumanEval/MBPP (saturated, trivial), AgentBench (stale, not code-focused).

## 5. Key Insight: No One Tests Framework-Guided AI

Every harness above measures **raw model capability** — none measure whether a development framework, linting configuration, or institutional knowledge system improves outcomes. The closest analog is Aider's benchmark, which tests the Aider agent vs raw model, but Aider is an agent (tool usage), not an institutional knowledge framework (rules, skills, conventions).

**COC's eval question is novel**: "Does the same model produce better code when institutional knowledge is injected into its context?" No existing benchmark answers this. We need to build it, but we can build it ON TOP of existing task sets (SWE-bench, BigCodeBench) rather than inventing tasks from scratch.

## Sources

- [SWE-bench](https://github.com/SWE-bench/SWE-bench) / [Leaderboard](https://www.swebench.com/)
- [SWE-bench Verified - OpenAI](https://openai.com/index/introducing-swe-bench-verified/) / [Epoch AI](https://epoch.ai/benchmarks/swe-bench-verified/)
- [SWE-bench Pro - Scale Labs](https://labs.scale.com/leaderboard/swe_bench_pro_public)
- [SWE-bench Live - Microsoft](https://github.com/microsoft/SWE-bench-Live)
- [BigCodeBench](https://github.com/bigcode-project/bigcodebench) / [Leaderboard](https://bigcode-bench.github.io/)
- [Aider Polyglot](https://github.com/Aider-AI/polyglot-benchmark) / [Leaderboards](https://aider.chat/docs/leaderboards/)
- [LiveCodeBench](https://github.com/LiveCodeBench/LiveCodeBench) / [Leaderboard](https://livecodebench.github.io/leaderboard.html)
- [CyberSecEval / PurpleLlama](https://github.com/meta-llama/PurpleLlama/tree/main/CybersecurityBenchmarks)
- [AutoPatchBench - Meta](https://engineering.fb.com/2025/04/29/ai-research/autopatchbench-benchmark-ai-powered-security-fixes/)
- [AgentBench](https://github.com/THUDM/AgentBench)
- [EvalPlus](https://github.com/evalplus/evalplus)
- [AI Coding Benchmarks 2026 Overview](https://www.morphllm.com/ai-coding-benchmarks-2026)
