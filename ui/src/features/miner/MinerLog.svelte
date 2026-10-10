<!-- W4 The miner log, newest first: chests, moments, raids, bets, drops, goals. -->
<script lang="ts">
  import { formatTime } from '../../lib/format';
  import { miner } from './miner.svelte';

  const lines = $derived([...miner.log].reverse());
</script>

<div class="log">
  {#each lines as l, i (l.t + ':' + i)}
    <p>
      <span class="when">{formatTime(new Date(l.t))}</span>
      {#if l.channel}<b>{l.channel}</b>{/if}
      <span>{l.text}</span>
    </p>
  {:else}
    <p class="empty">Nothing yet. Chests, raids, bets and drops show up here as they happen.</p>
  {/each}
</div>

<style>
  .log {
    max-width: 760px;
    font-size: 0.85rem;
  }
  p {
    display: flex;
    gap: 8px;
    margin: 0;
    padding: 4px 0;
    border-bottom: 1px solid var(--border);
  }
  .when {
    color: var(--dim);
    font-variant-numeric: tabular-nums;
  }
  .empty {
    border: 0;
    color: var(--dim);
  }
</style>
