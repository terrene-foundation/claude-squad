---
type: DISCOVERY
date: 2026-04-08
created_at: 2026-04-08T16:30:00+08:00
author: agent
session_id: unknown
session_turn: 15
project: claude-squad
topic: CC source code analysis reveals mtime-based credential reload — swap should work
phase: analyze
tags: [oauth, credentials, swap, claude-code, source-analysis, mtime, keychain]
---

## Summary

Source code analysis of Claude Code (from `~/repos/contrib/claude-code-source-code`)
reveals that CC already has a cross-process credential reload mechanism. The
previous hypothesis in 0014 (CC caches credentials forever in memory) is
**partially correct but incomplete** — CC does memoize with lodash `memoize`
(forever cache), but also has an mtime-based invalidation check that runs
before every API call.

## Key Code Paths

### 1. Credential Caching (auth.ts:1255)

```typescript
export const getClaudeAIOAuthTokens = memoize((): OAuthTokens | null => {
  const secureStorage = getSecureStorage();
  const storageData = secureStorage.read();
  return storageData?.claudeAiOauth;
});
```

Memoized with lodash `memoize` — cached forever once called. But see #2.

### 2. Mtime-Based Invalidation (auth.ts:1320)

```typescript
let lastCredentialsMtimeMs = 0;

async function invalidateOAuthCacheIfDiskChanged(): Promise<void> {
  const { mtimeMs } = await stat(
    join(getClaudeConfigHomeDir(), ".credentials.json"),
  );
  if (mtimeMs !== lastCredentialsMtimeMs) {
    lastCredentialsMtimeMs = mtimeMs;
    clearOAuthTokenCache(); // Clears BOTH memoize + 30s keychain TTL cache
  }
}
```

Added to fix CC-1096 / GH#24317 (cross-process credential staleness). This
function runs **before every API call** via `checkAndRefreshOAuthTokenIfNeeded()`
which is called inside `getAnthropicClient()` (client.ts:132).

### 3. Per-Turn Client Construction (withRetry.ts:185)

```typescript
let client: Anthropic | null = null  // Starts null per call
for (let attempt = 1; ...) {
  if (client === null || ...) {
    client = await getClient()  // Creates new Anthropic client
  }
}
```

A new Anthropic client is constructed per conversation turn (per `withRetry`
call). The client's `authToken` is set from `getClaudeAIOAuthTokens()?.accessToken`
at construction time (client.ts:303-304).

### 4. Storage Hierarchy (secureStorage/index.ts:10-11)

On macOS: `createFallbackStorage(macOsKeychainStorage, plainTextStorage)`

- Keychain is primary (30s TTL cache, cleared by `clearOAuthTokenCache`)
- `.credentials.json` is fallback (only if keychain read returns null)
- Mtime check triggers cache invalidation, but actual data comes from keychain

### 5. 401 Recovery (withRetry.ts:241-248)

On 401/403 errors, CC clears all caches, re-reads from storage, and either
uses a token refreshed by another process or forces its own refresh.

### 6. Triple-Check on Refresh (auth.ts:1453, 1474, 1519)

Before refreshing, CC reads tokens THREE times:

1. After mtime invalidation (line 1453)
2. Async re-read before lock (line 1474)
3. Inside the lock (line 1519)
   Each re-read clears caches first, so any external write (like csq swap)
   is picked up.

## Verified: csq's Writes Are Correct

Live verification on config-7:

- Keychain service name: `Claude Code-credentials-49132b3c` — matches CC's
  `getMacOsKeychainStorageServiceName()` formula
- Data format: `{claudeAiOauth: {...}}` wrapper, hex-encoded via `-X` flag
- `keychain == file (refreshToken): True`
- `keychain == file (accessToken): True`

## Verified: Refresh Tokens Are NOT Rotated

Compared all 7 `credentials/N.json` files against their live `.credentials.json`
counterparts. All refresh tokens match. Anthropic's OAuth server returns
`refresh_token: newRefreshToken = refreshToken` (client.ts:178) — the default
keeps the existing token if the server doesn't return a new one.

## Revised Assessment of 0014

The 0014 GAP entry hypothesized that CC caches forever and never re-reads.
This is **wrong** — CC has the mtime check that invalidates on every API call.
The swap mechanism should work. Possible explanations for the user's observation:

1. **Timing race**: CC was mid-refresh with the old account's tokens when csq
   wrote. The old refresh overwrites csq's swap. (Narrow window, but real.)
2. **Observational lag**: The statusline showed stale data from the pre-swap
   account, making the swap appear to fail. By the time `csq login` ran,
   the original swap may have already been working.
3. **Stale quota data triggering manual re-swap**: User saw account 4 at 100%
   in the statusline (stale from 3 hours ago) and interpreted this as the
   swap failing, when CC was actually using account 4's token for API calls.

## Diagnostic Added

`swap_to()` now includes:

- Immediate readback verification of `.credentials.json` after write
- A 2-second delayed verification (background thread) to catch CC overwriting
  the swap. If the delayed check finds a different refresh token, it prints
  `DIAG(+2s): .credentials.json OVERWRITTEN` with the overwriting account.

## For Discussion

1. Given that CC already checks `.credentials.json` mtime before every API call,
   should `CLAUDE.md:26` ("in-place rotation works") be restored to its original
   claim, with the caveat that there's a narrow timing race during concurrent
   token refreshes?

2. If the 2-second diagnostic never fires after weeks of use, does that confirm
   the observational-lag hypothesis? At that point, should the `live_credentials_account()`
   mismatch (the `⚠` flag) be downgraded from "swap may be stuck" to "statusline
   data is from the previous turn's API call"?

3. The `backsync` mechanism (statusline-quota.sh:45) syncs access tokens from
   `.credentials.json` back to `credentials/N.json`, but only when the refresh
   token matches. If Anthropic ever starts rotating refresh tokens, `backsync`
   silently stops working (no match → no sync). Should `backsync` have a
   fallback that matches on the `.csq-account` marker when refresh-token
   matching fails?
