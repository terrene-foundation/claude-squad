# Agent Orchestration Rules

## Scope

These rules govern when and how specialized agents are used in claude-squad.

## Recommended Delegations

### Code Review After Changes

After file modifications (Edit, Write), delegating to a code-review agent
is recommended when the change is non-trivial. Users may skip for trivial
edits or when explicitly told to.

### Security Review Before Commits

Before `git commit` on security-sensitive changes (OAuth flow, keychain
writes, atomic file handling), a security review is strongly recommended.

### Parallel Execution for Independent Operations

When multiple independent operations are needed, launch them in parallel
(e.g., reading several unrelated files, running multiple searches).

### Analysis Chain for Complex Features

For features with unclear requirements or multiple valid approaches:

1. Identify failure points (deep analysis)
2. Break down requirements
3. Choose an approach
4. Implement

## MUST Rules

### Zero-Tolerance Enforcement

Pre-existing failures, stubs, naive fallbacks, and error-hiding are BLOCKED.
See `zero-tolerance.md` and `no-stubs.md`. If you find it, you fix it.

## Cross-References

- `zero-tolerance.md` — what MUST be fixed (not reported)
- `no-stubs.md` — stub detection and enforcement
- `security.md` — security review checklist
- `git.md` — commit and branch workflow
