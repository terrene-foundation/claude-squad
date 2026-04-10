<script lang="ts">
  import AccountList from './lib/components/AccountList.svelte';
  import Header from './lib/components/Header.svelte';
</script>

<div class="app">
  <Header />
  <main>
    <AccountList />
  </main>
</div>

<style>
  .app {
    font-family: system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--bg-primary);
    color: var(--text-primary);
  }
  main {
    flex: 1;
    padding: 1rem;
    overflow-y: auto;
  }
</style>
