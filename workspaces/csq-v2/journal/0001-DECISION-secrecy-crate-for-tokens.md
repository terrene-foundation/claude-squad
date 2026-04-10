---
type: DECISION
date: 2026-04-10
created_at: 2026-04-10T18:00:00+08:00
author: agent
session_id: v2-scaffold
session_turn: 45
project: csq-v2
topic: Use secrecy::SecretString for token types instead of plain String
phase: implement
tags: [security, types, credentials]
---

# Use secrecy::SecretString for Token Types

## Decision

`AccessToken` and `RefreshToken` wrap `secrecy::SecretString` (v0.10), which provides zeroize-on-drop. Neither type implements `Serialize` — they cannot accidentally leak via IPC or logging.

## Alternatives Considered

- **Plain `String` with masked `Display`**: Prevents log leaks but doesn't zeroize memory on drop. Tokens linger in freed heap until the page is reused.
- **`zeroize::Zeroizing<String>`**: Zeroize without the `ExposeSecret` discipline. Callers can accidentally dereference to `&str` and pass it to logging macros.
- **`secrecy` v0.8 (`Secret<String>`)**: The v0.8 API was simpler but v0.10 renamed to `SecretString` = `SecretBox<str>`. We use v0.10 since it's current.

## Consequences

- Callers must explicitly call `.expose_secret()` to get the raw value. This makes every secret access auditable via grep.
- `Clone` must be manually implemented (reconstruct from exposed secret) since `SecretBox` is not `Clone`.
- The `secrecy` crate adds ~2KB to the binary.

## For Discussion

- If Anthropic changes token format to include non-UTF8 bytes, would `SecretBox<[u8]>` be needed? Current tokens are always ASCII.
- What happens to zeroize guarantees under `jemalloc` or custom allocators that may defer page return?
