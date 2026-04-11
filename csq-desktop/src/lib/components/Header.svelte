<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { homeDir, join } from '@tauri-apps/api/path';

  interface DaemonStatusView {
    running: boolean;
    pid: number | null;
  }

  let daemonRunning = $state(false);

  async function fetchDaemonStatus() {
    try {
      // Use `join` so the platform's path separator is honored.
      // Tauri 2.10's `homeDir()` returns a path without a trailing
      // separator, so naive concatenation produces an invalid path
      // like `/Users/esperie.claude/accounts` (see journal 0021).
      const home = await homeDir();
      const baseDir = await join(home, '.claude', 'accounts');
      const status = await invoke<DaemonStatusView>('get_daemon_status', { baseDir });
      daemonRunning = status.running;
    } catch {
      daemonRunning = false;
    }
  }

  $effect(() => {
    fetchDaemonStatus();
    const interval = setInterval(fetchDaemonStatus, 10000);
    return () => clearInterval(interval);
  });
</script>

<header>
  <div class="left">
    <h1>Claude Squad</h1>
    <span class="version">v2.0.0-alpha</span>
  </div>
  <div class="status">
    <span class="dot" class:running={daemonRunning}></span>
    <span class="label">{daemonRunning ? 'Daemon running' : 'Daemon stopped'}</span>
  </div>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.75rem 1rem;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    -webkit-app-region: drag;
  }
  .left { display: flex; align-items: center; gap: 0.5rem; }
  h1 { font-size: 0.9rem; font-weight: 600; margin: 0; }
  .version { font-size: 0.75rem; color: var(--text-secondary); }
  .status {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    -webkit-app-region: no-drag;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--red);
  }
  .dot.running { background: var(--green); }
  .label { font-size: 0.75rem; color: var(--text-secondary); }
</style>
