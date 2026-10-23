<!-- A channel's videos (1.21): past streams, highlights, uploads. At most 100 signed out.
     V107-Mv Each shows how far it was watched. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Video } from '../../ipc/bindings/Video';
  import type { VideoSort } from '../../ipc/bindings/VideoSort';
  import type { VodProgress } from '../../ipc/bindings/VodProgress';
  import { channelIpc, playerIpc } from '../../ipc/player';
  import { formatDate, formatShort } from '../../lib/format';
  import { clock } from '../player/time';
  import Card from './Card.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { login, onplay }: { login: string; onplay: (id: string) => void } = $props();

  const KINDS = [
    { value: 'all', label: 'All' },
    { value: 'archive', label: 'Past streams' },
    { value: 'highlight', label: 'Highlights' },
    { value: 'upload', label: 'Uploads' },
  ] as const;
  const SORTS = [
    { value: 'newest', label: 'Newest' },
    { value: 'views', label: 'Most viewed' },
  ] as const;

  let kind = $state<(typeof KINDS)[number]['value']>('all');
  let sort = $state<VideoSort>('newest');
  let videos = $state<Video[] | null>(null);
  let problem = $state<string | null>(null);
  let seen = $state.raw<Record<string, VodProgress>>({});

  $effect(() => {
    const [l, k, s] = [login, kind, sort];
    let alive = true;
    videos = null;
    problem = null;
    channelIpc
      .videos(l, k === 'all' ? null : k, s)
      .then((list) => {
        if (!alive) return;
        videos = list;
        void playerIpc
          .progress(list.map((v) => v.id))
          .then((ps) => alive && (seen = Object.fromEntries(ps.map((x) => [x.id, x]))))
          .catch(() => {});
      })
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });
</script>

<div class="page">
  <div class="filters">
    <Segmented wrap label="Kind" options={KINDS} value={kind} onchange={(v) => (kind = v)} />
    <Segmented label="Order" options={SORTS} value={sort} onchange={(v) => (sort = v)} />
  </div>
  {#if problem}
    <EmptyState icon="alert" text={problem} />
  {:else if !videos}
    <Loading cards={12} kind="media" label="Loading" />
  {:else if !videos.length}
    <EmptyState icon="film" text="No videos here." />
  {:else}
    <div class="grid">
      {#each videos as v (v.id)}
        <Card
          image={v.thumbnail}
          title={v.title}
          sub={[v.game, formatDate(new Date(v.publishedAt))].filter(Boolean).join(' · ')}
          corner={clock(v.length)}
          views={`${formatShort(v.views)} ${v.views === 1 ? 'view' : 'views'}`}
          badge={v.recording ? 'LIVE' : v.restricted ? 'Subs' : null}
          progress={seen[v.id]?.part}
          watched={seen[v.id]?.watched}
          onclick={() => onplay(v.id)}
        />
      {/each}
    </div>
  {/if}
</div>

<style>
  .page {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px;
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-bottom: 12px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px 12px;
  }
</style>
