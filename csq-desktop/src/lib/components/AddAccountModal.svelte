<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { openUrl } from '@tauri-apps/plugin-opener';
  import { homeDir, join } from '@tauri-apps/api/path';

  // ── Props ─────────────────────────────────────────────────
  let {
    isOpen,
    nextAccountId,
    onClose,
    onAccountAdded,
  }: {
    isOpen: boolean;
    nextAccountId: number;
    onClose: () => void;
    onAccountAdded: () => void;
  } = $props();

  // ── Types ─────────────────────────────────────────────────
  interface ProviderView {
    id: string;
    name: string;
    auth_type: 'oauth' | 'bearer' | 'none';
    default_base_url: string | null;
    default_model: string;
  }

  interface ClaudeLoginView {
    auth_url: string;
    state: string;
    account: number;
    expires_in_secs: number;
  }

  // ── Local state ───────────────────────────────────────────
  //
  // Claude OAuth paste-code flow (Anthropic no longer supports
  // loopback redirect for this client_id — the user has to copy
  // the authorization code from Anthropic's callback page and
  // paste it back into the app):
  //   1. `picker`        — user picks a provider
  //   2. `paste-code`    — browser is open, user pastes the code
  //   3. `exchanging`    — submitting code to Anthropic
  //   4. `success`       — account added
  //   5. `error`         — something failed; user can retry
  //
  // Bearer-key flow (MiniMax, Z.AI):
  //   1. `picker`        — user picks a provider
  //   2. `bearer-form`   — user pastes an API key
  //   3. `success` / `error`
  type Step =
    | { kind: 'picker' }
    | {
        kind: 'paste-code';
        login: ClaudeLoginView;
        code: string;
        error: string | null;
      }
    | { kind: 'exchanging'; account: number }
    | {
        kind: 'bearer-form';
        provider: ProviderView;
        key: string;
        submitting: boolean;
        error: string | null;
      }
    | { kind: 'success'; message: string }
    | { kind: 'error'; message: string };

  let step = $state<Step>({ kind: 'picker' });
  let providers = $state<ProviderView[]>([]);
  let providersError = $state<string | null>(null);

  // ── Provider fetch ────────────────────────────────────────
  async function loadProviders() {
    try {
      providers = await invoke<ProviderView[]>('list_providers');
      providersError = null;
    } catch (e) {
      providersError = String(e);
    }
  }

  // Reset to picker whenever the modal re-opens. Cancel any
  // in-flight PKCE state when the modal closes mid-flow so the
  // state store doesn't fill with abandoned entries.
  $effect(() => {
    if (isOpen) {
      step = { kind: 'picker' };
      let cancelled = false;
      (async () => {
        if (!cancelled) await loadProviders();
      })();
      return () => { cancelled = true; };
    }
  });

  async function getBaseDir(): Promise<string> {
    // `join` honors the platform path separator and Tauri 2.10's
    // `homeDir()` has no trailing separator, so naive string
    // concatenation would produce `/Users/x.claude/accounts`.
    const home = await homeDir();
    return await join(home, '.claude', 'accounts');
  }

  // ── Provider pick ─────────────────────────────────────────
  async function pickProvider(provider: ProviderView) {
    if (provider.auth_type === 'oauth') {
      await startClaudeOAuth();
    } else if (provider.auth_type === 'bearer') {
      step = {
        kind: 'bearer-form',
        provider,
        key: '',
        submitting: false,
        error: null,
      };
    }
  }

  // ── Claude OAuth (in-process paste-code flow) ─────────────
  //
  // Anthropic's current OAuth flow for this client_id uses a
  // paste-code redirect: the authorize URL has
  // `redirect_uri=https://platform.claude.com/oauth/code/callback`,
  // the hosted page shows the user an authorization code, the
  // user pastes it back into csq, and the backend exchanges it
  // for tokens.
  //
  // The previous `start_claude_login` command shelled out to
  // `claude auth login`, which fails in GUI context because
  // Finder-launched apps don't inherit the user's shell PATH.
  // The new flow uses `begin_claude_login` + `submit_oauth_code`
  // entirely in-process.
  async function startClaudeOAuth() {
    try {
      const login = await invoke<ClaudeLoginView>('begin_claude_login', {
        account: nextAccountId,
      });
      // Open the authorize URL in the user's default browser.
      await openUrl(login.auth_url);
      step = {
        kind: 'paste-code',
        login,
        code: '',
        error: null,
      };
    } catch (e) {
      step = { kind: 'error', message: String(e) };
    }
  }

  async function submitOAuthCode() {
    if (step.kind !== 'paste-code') return;
    const current = step;
    const code = current.code.trim();
    if (!code) {
      step = { ...current, error: 'Authorization code must not be empty' };
      return;
    }

    step = { kind: 'exchanging', account: current.login.account };
    try {
      const baseDir = await getBaseDir();
      const account = await invoke<number>('submit_oauth_code', {
        baseDir,
        stateToken: current.login.state,
        code,
      });
      onAccountAdded();
      step = {
        kind: 'success',
        message: `Account ${account} added successfully.`,
      };
    } catch (e) {
      step = {
        kind: 'paste-code',
        login: current.login,
        code: current.code,
        error: String(e),
      };
    }
  }

  async function cancelPasteCode() {
    if (step.kind === 'paste-code') {
      // Best-effort: consume the pending state so it doesn't linger.
      try {
        await invoke('cancel_login', { stateToken: step.login.state });
      } catch (_) {
        // Silently ignore — the server will expire the state TTL anyway.
      }
    }
    step = { kind: 'picker' };
  }

  // ── Bearer key flow ───────────────────────────────────────
  async function submitBearerKey() {
    if (step.kind !== 'bearer-form') return;
    const providerStep = step;
    if (!providerStep.key.trim()) {
      step = { ...providerStep, error: 'API key must not be empty' };
      return;
    }

    step = { ...providerStep, submitting: true, error: null };
    try {
      const baseDir = await getBaseDir();
      const fingerprint = await invoke<string>('set_provider_key', {
        baseDir,
        providerId: providerStep.provider.id,
        key: providerStep.key.trim(),
      });
      onAccountAdded();
      step = {
        kind: 'success',
        message: `${providerStep.provider.name} key saved (${fingerprint}).`,
      };
    } catch (e) {
      step = { ...providerStep, submitting: false, error: String(e) };
    }
  }

  // ── Close behavior ────────────────────────────────────────
  async function handleClose() {
    onClose();
  }
</script>

{#if isOpen}
  <div
    class="backdrop"
    onclick={handleClose}
    onkeydown={(e) => {
      if (e.key === 'Escape') handleClose();
    }}
    role="button"
    tabindex="-1"
  >
    <div
      class="modal"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
      aria-labelledby="add-account-title"
      tabindex="-1"
    >
      <header>
        <h2 id="add-account-title">Add Account</h2>
        <button class="close" onclick={handleClose} aria-label="Close">×</button>
      </header>

      <div class="body">
        {#if step.kind === 'picker'}
          <p class="lede">Pick a provider to connect to account slot #{nextAccountId}.</p>
          {#if providersError}
            <div class="error-banner">Could not load providers: {providersError}</div>
          {/if}
          <div class="provider-grid">
            {#each providers as provider (provider.id)}
              <button class="provider-card" onclick={() => pickProvider(provider)}>
                <div class="provider-name">{provider.name}</div>
                <div class="provider-meta">
                  {provider.auth_type === 'oauth' ? 'Sign in with Anthropic' : 'Paste an API key'}
                </div>
                {#if provider.default_model}
                  <div class="provider-model">{provider.default_model}</div>
                {/if}
              </button>
            {/each}
          </div>
        {:else if step.kind === 'paste-code'}
          <p class="lede">
            Signing in to account #{step.login.account}…
          </p>
          <p class="hint">
            A browser window should open to Anthropic. Sign in, then
            copy the authorization code from the callback page and
            paste it below.
          </p>
          <p class="hint">
            If the browser didn't open, <a
              href={step.login.auth_url}
              target="_blank"
              rel="noopener noreferrer">open the sign-in URL manually</a
            >.
          </p>
          <label class="field">
            <span>Authorization code</span>
            <input
              type="text"
              bind:value={step.code}
              placeholder="Paste the code from Anthropic's page"
              autocomplete="off"
              spellcheck="false"
            />
          </label>
          {#if step.error}
            <div class="error-banner">{step.error}</div>
          {/if}
          <div class="actions">
            <button class="secondary" onclick={cancelPasteCode}>Cancel</button>
            <button
              class="primary"
              onclick={submitOAuthCode}
              disabled={!step.code.trim()}
            >
              Complete sign-in
            </button>
          </div>
        {:else if step.kind === 'exchanging'}
          <p class="lede">
            Exchanging the code for account #{step.account}…
          </p>
          <p class="hint">Talking to Anthropic. This usually takes a second.</p>
        {:else if step.kind === 'bearer-form'}
          <p class="lede">Paste your {step.provider.name} API key.</p>
          <label class="field">
            <span>API key</span>
            <input
              type="password"
              bind:value={step.key}
              placeholder="sk-…"
              disabled={step.submitting}
              autocomplete="off"
            />
          </label>
          {#if step.provider.default_base_url}
            <p class="hint">
              Using default endpoint: <code>{step.provider.default_base_url}</code>
            </p>
          {/if}
          {#if step.error}
            <div class="error-banner">{step.error}</div>
          {/if}
          <div class="actions">
            <button class="secondary" onclick={() => (step = { kind: 'picker' })}>Back</button>
            <button class="primary" onclick={submitBearerKey} disabled={step.submitting}>
              {step.submitting ? 'Saving…' : 'Save key'}
            </button>
          </div>
        {:else if step.kind === 'success'}
          <div class="success-banner">{step.message}</div>
          <div class="actions">
            <button class="primary" onclick={handleClose}>Done</button>
          </div>
        {:else if step.kind === 'error'}
          <div class="error-banner">{step.message}</div>
          <div class="actions">
            <button class="secondary" onclick={() => (step = { kind: 'picker' })}>Try again</button>
            <button class="danger" onclick={handleClose}>Close</button>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    cursor: default;
  }
  .modal {
    background: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 8px;
    width: min(480px, 90vw);
    max-height: 90vh;
    overflow-y: auto;
    box-shadow: 0 20px 40px rgba(0, 0, 0, 0.35);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.85rem 1rem;
    border-bottom: 1px solid var(--border);
  }
  header h2 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
  }
  .close {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 1.4rem;
    line-height: 1;
    cursor: pointer;
    padding: 0 0.25rem;
  }
  .close:hover {
    color: var(--text-primary);
  }
  .body {
    padding: 1rem;
  }
  .lede {
    margin: 0 0 0.75rem 0;
    font-size: 0.9rem;
  }
  .hint {
    margin: 0.25rem 0;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .hint code {
    background: var(--bg-tertiary);
    padding: 0.1em 0.35em;
    border-radius: 3px;
    font-size: 0.95em;
  }
  .provider-grid {
    display: grid;
    gap: 0.5rem;
  }
  .provider-card {
    text-align: left;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 0.75rem;
    cursor: pointer;
    color: inherit;
    font: inherit;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    transition: border-color 0.15s;
  }
  .provider-card:hover {
    border-color: var(--accent);
  }
  .provider-name {
    font-weight: 600;
    font-size: 0.95rem;
  }
  .provider-meta {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .provider-model {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-family: ui-monospace, monospace;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    margin: 0.5rem 0;
  }
  .field span {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }
  .field input {
    padding: 0.5rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: inherit;
    font: inherit;
    font-family: ui-monospace, monospace;
    font-size: 0.85rem;
  }
  .field input:focus {
    outline: 2px solid var(--accent);
    outline-offset: -1px;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
    margin-top: 0.85rem;
  }
  .actions button {
    padding: 0.45rem 0.85rem;
    border-radius: 4px;
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    border: 1px solid var(--border);
    background: var(--bg-secondary);
    color: inherit;
  }
  .actions button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: white;
  }
  .actions button.primary:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }
  .actions button.danger {
    color: var(--red);
    border-color: var(--red);
  }
  .error-banner {
    background: rgba(255, 80, 80, 0.12);
    border: 1px solid var(--red);
    border-radius: 4px;
    padding: 0.55rem 0.7rem;
    color: var(--red);
    font-size: 0.85rem;
    margin: 0.5rem 0;
  }
  .success-banner {
    background: rgba(80, 200, 120, 0.12);
    border: 1px solid #4caf50;
    border-radius: 4px;
    padding: 0.55rem 0.7rem;
    color: #4caf50;
    font-size: 0.9rem;
  }

  .url-box {
    width: 100%;
    padding: 0.5rem;
    margin: 0.35rem 0;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: inherit;
    font: inherit;
    font-family: ui-monospace, monospace;
    font-size: 0.75rem;
    resize: vertical;
    word-break: break-all;
  }
</style>
