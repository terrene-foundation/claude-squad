<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { homeDir } from '@tauri-apps/api/path';
  import UsageBar from './UsageBar.svelte';
  import TokenBadge from './TokenBadge.svelte';

  interface AccountView {
    id: number;
    label: string;
    source: string;
    has_credentials: boolean;
    five_hour_pct: number;
    seven_day_pct: number;
    updated_at: number;
    token_status: string;
    expires_in_secs: number | null;
  }

  let accounts = $state<AccountView[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);
  let loginMessage = $state<string | null>(null);

  async function getBaseDir(): Promise<string> {
    const home = await homeDir();
    return home + '.claude/accounts';
  }

  async function fetchAccounts() {
    try {
      const baseDir = await getBaseDir();
      accounts = await invoke<AccountView[]>('get_accounts', { baseDir });
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleAddAccount() {
    const nextId = accounts.length > 0
      ? Math.max(...accounts.map(a => a.id)) + 1
      : 1;
    try {
      loginMessage = await invoke<string>('start_login', { account: nextId });
    } catch (e) {
      loginMessage = String(e);
    }
  }

  async function handleSwap(accountId: number) {
    try {
      const baseDir = await getBaseDir();
      await invoke('swap_account', { baseDir, target: accountId });
      await fetchAccounts();
    } catch (e) {
      error = String(e);
    }
  }

  // Initial fetch + 5-second poll
  $effect(() => {
    fetchAccounts();
    const interval = setInterval(fetchAccounts, 5000);
    return () => clearInterval(interval);
  });
</script>

{#if loading}
  <div class="loading">Loading accounts...</div>
{:else if error}
  <div class="error">{error}</div>
{:else if accounts.length === 0}
  <div class="empty">
    <p>No accounts configured.</p>
    <p class="hint">Run <code>csq login 1</code> to add your first account.</p>
  </div>
{:else}
  <div class="account-list">
    {#each accounts as account (account.id)}
      <button class="account-card" class:no-creds={!account.has_credentials}
              onclick={() => handleSwap(account.id)}>
        <div class="account-header">
          <span class="account-id">#{account.id}</span>
          <span class="account-label">{account.label}</span>
          <TokenBadge status={account.token_status} expiresSecs={account.expires_in_secs} />
        </div>
        <div class="usage-bars">
          <UsageBar label="5h" pct={account.five_hour_pct} />
          <UsageBar label="7d" pct={account.seven_day_pct} />
        </div>
      </button>
    {/each}
  </div>
{/if}

{#if loginMessage}
  <div class="login-prompt">
    <p>{loginMessage}</p>
    <button class="dismiss" onclick={() => loginMessage = null}>Dismiss</button>
  </div>
{/if}

<div class="actions">
  <button class="add-account" onclick={handleAddAccount}>+ Add Account</button>
</div>

<style>
  .account-list { display: flex; flex-direction: column; gap: 0.5rem; }
  .account-card {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    text-align: left;
    color: inherit;
    font: inherit;
    width: 100%;
    transition: border-color 0.15s;
  }
  .account-card:hover { border-color: var(--accent); }
  .account-card.no-creds { opacity: 0.5; }
  .account-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .account-id { font-weight: 700; font-size: 0.85rem; color: var(--text-secondary); }
  .account-label { flex: 1; font-weight: 500; }
  .usage-bars { display: flex; gap: 1rem; }
  .loading, .error, .empty { padding: 2rem; text-align: center; }
  .error { color: var(--red); }
  .hint { font-size: 0.85rem; color: var(--text-secondary); }
  code { background: var(--bg-tertiary); padding: 0.15em 0.4em; border-radius: 3px; font-size: 0.85em; }
  .actions { margin-top: 0.75rem; }
  .add-account {
    width: 100%;
    padding: 0.6rem;
    background: transparent;
    border: 1px dashed var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
    cursor: pointer;
    font: inherit;
    font-size: 0.85rem;
    transition: border-color 0.15s, color 0.15s;
  }
  .add-account:hover { border-color: var(--accent); color: var(--accent); }
  .login-prompt {
    margin-top: 0.75rem;
    padding: 0.75rem;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    font-size: 0.85rem;
  }
  .dismiss {
    margin-top: 0.5rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-secondary);
    padding: 0.3rem 0.6rem;
    cursor: pointer;
    font: inherit;
    font-size: 0.8rem;
  }
</style>
