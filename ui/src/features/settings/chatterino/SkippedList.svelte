<!-- V105-16 What can't be imported and why, the first 20 and a count of the rest. V111-60
     Shared by every Chatterino review. -->
<script lang="ts">
  import type { Skipped } from '../../../ipc/bindings/Skipped';
  import { formatNumber } from '../../../lib/format';

  const MAX_LINES = 20;

  interface Props {
    list: Skipped[];
  }

  let { list }: Props = $props();

  const shown = $derived(list.slice(0, MAX_LINES));
  const more = $derived(Math.max(0, list.length - MAX_LINES));
</script>

{#if list.length}
  <p class="head">{formatNumber(list.length)} can’t be imported:</p>
  <ul>
    {#each shown as s, i (i)}
      <li><code>{s.pattern}</code> <span class="why">{s.reason}</span></li>
    {/each}
  </ul>
  {#if more}<p class="why">and {formatNumber(more)} more</p>{/if}
{/if}

<style>
  .head {
    margin: 8px 0;
    font-weight: 600;
  }
  .why {
    margin: 6px 0;
    font-size: var(--text-hint);
    color: var(--dim);
  }
  ul {
    margin: 6px 0 0;
    padding: 0;
    list-style: none;
    display: grid;
    gap: 4px;
    font-size: var(--text-sub);
  }
  li {
    overflow-wrap: anywhere;
  }
  code {
    font-family: var(--font-mono);
    font-size: 0.78rem;
  }
</style>
