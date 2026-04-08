---
type: DECISION
title: Multi-provider support with Ollama, 3P model primers, and COC compliance benchmarks
date: 2026-04-09
---

## Context

User needed to run CC against local models when all Claude Max accounts hit rate limits. Ollama supports the Anthropic messages API natively (https://docs.ollama.com/integrations/claude-code). Non-Anthropic models receive CC's full system prompt but don't reliably follow COC rules.

## Decision

1. **Add Ollama provider**: `csq setkey ollama` + `csq run N -p ollama`. No API key needed. Keyless provider support added to setkey flow.
2. **Add 3P model primers**: Prepend (246 tok) + append (1,130 tok) system prompts bookending the context for primacy-recency effect. Only applied to non-Claude profiles (mm/zai/ollama). Primers reinforce tool usage, CLAUDE.md compliance, proactive agent/skill invocation, hooks.
3. **Update Z.AI** from glm-4.6 to glm-4.7.
4. **COC compliance benchmark**: Real CC instances via `claude --print` against coc-env. Tests CC platform compliance (tool usage, agents, skills) and COC governance enforcement (rule-violating prompts).

## Results

- Claude Opus: 7/7 governance, 15/15 platform, 18s/task
- MiniMax M2.7: 15/15 platform, 20s/task (governance TBD)
- gemma4: 4/7 governance, 15/15 platform, 89s/task
- qwen3.5: 3/7 governance, 8/15 platform, 212s/task (frequent timeouts)

## Key Finding

The governance gap is real: all non-Claude models fail framework-first (checking specialists before writing code). gemma4 is the best local option but still misses subtle constraints. Claude remains the only model that enforces all COC rules.
