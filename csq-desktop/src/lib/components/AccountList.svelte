<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { homeDir } from '@tauri-apps/api/path';
  import UsageBar from './UsageBar.svelte';

  interface AccountView {
    id: number;
    label: string;
    source: string;
    has_credentials: boolean;
    five_hour_pct: number;
    seven_day_pct: number;
    updated_at: number;
  }

  let accounts = $state<AccountView[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(true);

  async function fetchAccounts() {
    try {
      const home = await homeDir();
      const baseDir = home + '.claude/accounts';
      accounts = await invoke<AccountView[]>('get_accounts', { baseDir });
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
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
      <div class="account-card" class:no-creds={!account.has_credentials}>
        <div class="account-header">
          <span class="account-id">#{account.id}</span>
          <span class="account-label">{account.label}</span>
          <span class="account-source">{account.source}</span>
        </div>
        <div class="usage-bars">
          <UsageBar label="5h" pct={account.five_hour_pct} />
          <UsageBar label="7d" pct={account.seven_day_pct} />
        </div>
      </div>
    {/each}
  </div>
{/if}

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
  }
  .account-card.no-creds { opacity: 0.5; }
  .account-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .account-id { font-weight: 700; font-size: 0.85rem; color: var(--text-secondary); }
  .account-label { flex: 1; font-weight: 500; }
  .account-source { font-size: 0.75rem; color: var(--text-tertiary); text-transform: uppercase; }
  .usage-bars { display: flex; gap: 1rem; }
  .loading, .error, .empty { padding: 2rem; text-align: center; }
  .error { color: var(--red); }
  .hint { font-size: 0.85rem; color: var(--text-secondary); }
  code { background: var(--bg-tertiary); padding: 0.15em 0.4em; border-radius: 3px; font-size: 0.85em; }
</style>
