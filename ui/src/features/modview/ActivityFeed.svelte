<!-- 1.48 The activity feed: follows, raids, unban requests and shield changes, plus
     suspicious-user messages (M6), newest first. -->
<script lang="ts">
  import type { FeedItem } from '../../ipc/bindings/FeedItem';
  import type { SuspiciousItem } from '../../ipc/bindings/SuspiciousItem';
  import { formatTime } from '../../lib/format';

  let { items, suspicious }: { items: FeedItem[]; suspicious: SuspiciousItem[] } = $props();

  type Row = { key: string; ts: number; text: string; tag: string };

  const rows = $derived.by<Row[]>(() => {
    const a: Row[] = items.map((i) => ({ key: i.id, ts: i.ts, text: i.text, tag: i.kind }));
    const s: Row[] = suspicious.map((i) => ({
      key: `s-${i.userId}-${i.ts}`,
      ts: i.ts,
      text: `${i.display} (${i.status === 'restricted' ? 'restricted' : 'monitored'}): ${i.text}`,
      tag: 'suspicious',
    }));
    return [...a, ...s].sort((x, y) => y.ts - x.ts).slice(0, 200);
  });
</script>

{#if rows.length === 0}
  <p class="empty">Nothing yet.</p>
{:else}
  <ul>
    {#each rows as r (r.key)}
      <li class="tag-{r.tag}">
        <span class="t">{formatTime(new Date(r.ts))}</span>
        <span class="text">{r.text}</span>
      </li>
    {/each}
  </ul>
{/if}

<style>
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
    border-left: 2px solid transparent;
  }
  li.tag-suspicious {
    border-left-color: var(--yellow);
  }
  li.tag-mod_action {
    border-left-color: var(--accent);
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
