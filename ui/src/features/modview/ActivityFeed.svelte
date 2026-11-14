<!-- 1.48 The activity feed: follows and unban requests, plus suspicious-user messages (M6),
     newest first. V103-15 Mod actions have their own panel; the list is shared with it.
     V111-16 A line's person opens their usercard in this channel. -->
<script lang="ts">
  import FeedList from '../../components/FeedList.svelte';
  import UserLink from '../../components/UserLink.svelte';
  import type { FeedItem } from '../../ipc/bindings/FeedItem';
  import type { SuspiciousItem } from '../../ipc/bindings/SuspiciousItem';
  import { splitNames } from '../chat/noticeNames';

  let { items, suspicious, login = '' }: { items: FeedItem[]; suspicious: SuspiciousItem[]; /** The channel. */ login?: string } = $props();

  type Row = { key: string; ts: number; text: string; kind: string; who?: { name: string; login: string } };

  const rows = $derived.by<Row[]>(() => {
    const a: Row[] = items.map((i) => ({ key: i.id, ts: i.ts, text: i.text, kind: i.kind, ...(i.user && { who: { name: i.user, login: i.user } }) }));
    const s: Row[] = suspicious.map((i) => ({
      key: `s-${i.userId}-${i.ts}`,
      ts: i.ts,
      text: `${i.display} (${i.status === 'restricted' ? 'restricted' : 'monitored'}): ${i.text}`,
      kind: 'suspicious',
      who: { name: i.display, login: i.login },
    }));
    return [...a, ...s].sort((x, y) => y.ts - x.ts).slice(0, 200);
  });

  const parts = (r: Row) => (r.who && login ? splitNames(r.text, [r.who]) : [{ text: r.text }]);
</script>

<FeedList {rows} icon="activity" empty="Nothing yet.">
  {#snippet row(r)}{#each parts(r) as p}{#if 'login' in p && p.login}<UserLink channel={login} login={p.login}>{p.text}</UserLink>{:else}{p.text}{/if}{/each}{/snippet}
</FeedList>
