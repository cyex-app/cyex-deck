<!-- V1051-02/03 The fixed view: pick your 7TV paint and badge. -->
<script lang="ts">
  import Segmented from '../../components/Segmented.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import CosmeticsPanel from './CosmeticsPanel.svelte';

  let { onclose }: { onclose?: () => void } = $props();

  let chosen = $state<string | null>(null);
  const picked = $derived(
    accounts.list.find((a) => a.userId === chosen)?.userId ?? accounts.current?.userId ?? accounts.list[0]?.userId ?? null,
  );
</script>

<div class="view">
  <header>
    <h1>7TV paint and badge</h1>
    {#if accounts.list.length > 1 && picked}
      <Segmented
        label="Account"
        value={picked}
        options={accounts.list.map((a) => ({ value: a.userId, label: a.displayName }))}
        onchange={(id) => (chosen = id)}
      />
    {/if}
  </header>
  <div class="body">
    {#if picked}
      {#key picked}
        <CosmeticsPanel account={picked} compact={false} {onclose} />
      {/key}
    {/if}
  </div>
</div>

<style>
  .view {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 4px 0 0;
    font-size: 1.05rem;
  }
  .body {
    flex: 1;
    width: 100%;
    max-width: 1100px;
    min-height: 0;
    margin: 0 auto;
    padding: 16px 20px;
    box-sizing: border-box;
  }
</style>
