<!-- 1.48 The mod-action log: bans, timeouts, deletes and mode changes from EventSub, with
     a filter box. Shared chat actions carry their channel in the line. -->
<script lang="ts">
  import type { FeedItem } from '../../ipc/bindings/FeedItem';
  import { formatTime } from '../../lib/format';

  let { items }: { items: FeedItem[] } = $props();

  let query = $state('');
  const shown = $derived(
    query.trim() ? items.filter((i) => i.text.toLowerCase().includes(query.trim().toLowerCase())) : items,
  );
</script>

<div class="bar">
  <input placeholder="Filter actions" bind:value={query} />
</div>
{#if shown.length === 0}
  <p class="empty">No mod actions yet.</p>
{:else}
  <ul>
    {#each shown as i (i.id)}
      <li>
        <span class="t">{formatTime(new Date(i.ts))}</span>
        <span class="text">{i.text}</span>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .bar {
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
  }
  input {
    width: 100%;
    padding: 4px 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  ul {
    margin: 0;
    padding: 4px 0;
    list-style: none;
  }
  li {
    display: flex;
    gap: 6px;
    padding: 3px 10px;
    font-size: 0.82rem;
  }
  .t {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    flex-shrink: 0;
  }
  .text {
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .empty {
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
