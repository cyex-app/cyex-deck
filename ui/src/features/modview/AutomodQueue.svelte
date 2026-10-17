<!-- 1.56 AutoMod queue: messages held for review, with Allow and Deny. The list is kept
     live from EventSub; resolving drops the row. -->
<script lang="ts">
  import { uniqueKeys } from '../../lib/keys';
  import HeldRow from '../feeds/HeldRow.svelte';
  import type { HeldItem } from '../../ipc/bindings/HeldItem';

  let { account, login, held }: { account: string; login: string; held: HeldItem[] } = $props();
  const keys = $derived(uniqueKeys(held, (h) => h.messageId));
</script>

{#if held.length === 0}
  <p class="empty">Nothing held.</p>
{:else}
  <ul>
    {#each held as h, i (keys[i])}
      <li><HeldRow {account} {login} item={h} /></li>
    {/each}
  </ul>
{/if}

<style>
  ul {
    margin: 0;
    padding: 6px;
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  li {
    padding: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .empty {
    padding: 12px;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
