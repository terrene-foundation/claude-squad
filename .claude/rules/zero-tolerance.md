# Zero-Tolerance Enforcement Rules

## Scope

These rules apply to ALL sessions, ALL agents, ALL code changes. They are
ABSOLUTE. NO flexibility.

## RULE 1: Pre-Existing Failures MUST Be Resolved

When tests, validation, or analysis reveals a pre-existing failure:
**YOU MUST FIX IT.** "Not introduced in this session" is not an acceptable
response. If you found it, you own it.

**Required**: diagnose root cause → implement fix → write regression test
→ verify → commit.

**BLOCKED responses**:

- "Pre-existing issue, out of scope"
- "Noting as a known issue for future resolution"
- Any acknowledgment without a fix

**Exception**: User explicitly says "skip" or "ignore."

## RULE 2: No Stubs, Placeholders, Deferred Implementation

Stubs are BLOCKED. See `no-stubs.md` for detection patterns and enforcement.
`validate-workflow.js` exits with code 2 on detection.

## RULE 3: No Naive Fallbacks or Error Hiding

`except: pass`, `return None` without logging, silent discards — BLOCKED.
See `no-stubs.md` Section 3.

## RULE 4: No Workarounds for Upstream Bugs

When you hit a bug in an upstream dependency (Claude Code CLI, Anthropic
OAuth endpoint, macOS `security` tool): reproduce it, document it, and
file an upstream issue. Do NOT re-implement the upstream's job yourself.

**BLOCKED**: naive re-implementations, post-processing to "fix" upstream
output, downgrading to avoid bugs.

## Language

Every "MUST" means MUST. Every "BLOCKED" means the operation does not
proceed. Every "NO" means NO.
