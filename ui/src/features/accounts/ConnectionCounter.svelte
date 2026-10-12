<!-- Joined-chat counter in the top bar (1.13, D7). One account reads "37/100"; several
     show the total, with each account's share in the tooltip. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { accounts } from '../../lib/accounts.svelte';

  /** Turns yellow from here on. */
  const NEAR_LIMIT = 0.9;

  const counts = $derived(accounts.connections);
  const total = $derived(counts.reduce((n, c) => n + c.counted, 0));
  const near = $derived(counts.some((c) => c.counted >= c.limit * NEAR_LIMIT));
  const text = $derived(counts.length === 1 ? `${total}/${counts[0].limit}` : String(total));
  const label = $derived(
    counts.length === 1
      ? 'Joined chats'
      : `Joined chats: ${counts
          .map((c) => `${accounts.list.find((a) => a.userId === c.userId)?.displayName ?? c.userId} ${c.counted}/${c.limit}`)
          .join(', ')}`,
  );
</script>

{#if counts.length}
  <span
    class="counter"
    class:near
    use:tooltip={{ label, hint: "Twitch allows 100 per account at a time. Chats you moderate don't count." }}
  >
    <Icon name="chat" size={13} />
    {text}
  </span>
{/if}

<style>
  .counter {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    height: 24px;
    padding: 0 8px;
    margin-right: 4px;
    border-radius: var(--radius);
    color: var(--dim);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  .counter:hover {
    background: var(--surface-hover);
    color: var(--muted);
  }
  .near {
    color: var(--yellow);
  }
</style>
