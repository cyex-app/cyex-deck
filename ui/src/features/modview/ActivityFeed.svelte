<!-- 1.48 The activity feed: follows and unban requests, plus suspicious-user messages (M6),
     newest first. V103-15 Mod actions have their own panel; the list is shared with it. -->
<script lang="ts">
  import FeedList from '../../components/FeedList.svelte';
  import type { FeedItem } from '../../ipc/bindings/FeedItem';
  import type { SuspiciousItem } from '../../ipc/bindings/SuspiciousItem';

  let { items, suspicious }: { items: FeedItem[]; suspicious: SuspiciousItem[] } = $props();

  type Row = { key: string; ts: number; text: string; kind: string };

  const rows = $derived.by<Row[]>(() => {
    const a: Row[] = items.map((i) => ({ key: i.id, ts: i.ts, text: i.text, kind: i.kind }));
    const s: Row[] = suspicious.map((i) => ({
      key: `s-${i.userId}-${i.ts}`,
      ts: i.ts,
      text: `${i.display} (${i.status === 'restricted' ? 'restricted' : 'monitored'}): ${i.text}`,
      kind: 'suspicious',
    }));
    return [...a, ...s].sort((x, y) => y.ts - x.ts).slice(0, 200);
  });
</script>

<FeedList {rows} empty="Nothing yet.">
  {#snippet row(r)}{r.text}{/snippet}
</FeedList>
