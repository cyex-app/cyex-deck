<!-- V110-U65 Reconnects per socket in the last hour and the last error, for when something
     feels flaky. Read when opened and on Refresh; nothing polls. -->
<script lang="ts">
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import { accountsIpc } from '../../../ipc/accounts';
  import type { LinkHealth } from '../../../ipc/bindings/LinkHealth';
  import { healthRows } from './connectionHealth';

  let open = $state(false);
  let list = $state<LinkHealth[]>([]);

  async function load() {
    list = await accountsIpc.health().catch(() => []);
  }
  function toggle() {
    open = !open;
    if (open) void load();
  }
  const rows = $derived(healthRows(list, Date.now()));
</script>

<Row label="Connection health" hint="Reconnects in the last hour per socket, and why the last one happened.">
  {#if open}<Button small onclick={() => void load()}>Refresh</Button>{/if}
  <Button small onclick={toggle}>{open ? 'Hide' : 'Show'}</Button>
</Row>
{#if open}
  <table class="health">
    <thead><tr><th>Link</th><th class="n">Last hour</th><th>Last drop</th><th>Why</th></tr></thead>
    <tbody>
      {#each rows as r (r.label)}
        <tr><td>{r.label}</td><td class="n">{r.count}</td><td>{r.when}</td><td>{r.why}</td></tr>
      {:else}
        <tr><td colspan="4" class="none">No drops since start.</td></tr>
      {/each}
    </tbody>
  </table>
{/if}

<style>
  .health {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--text-sub);
    font-variant-numeric: tabular-nums;
    margin: 4px 0 8px;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--muted);
    padding: 4px 8px 4px 0;
  }
  td {
    padding: 4px 8px 4px 0;
    border-top: 1px solid var(--border);
    color: var(--text);
  }
  .n {
    text-align: right;
  }
  .none {
    color: var(--muted);
  }
</style>
