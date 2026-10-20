<!-- A channel's top clips (1.21). Signed out, Twitch lists at most 100 per filter. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Clip } from '../../ipc/bindings/Clip';
  import type { ClipPeriod } from '../../ipc/bindings/ClipPeriod';
  import type { ClipSort } from '../../ipc/bindings/ClipSort';
  import { channelIpc } from '../../ipc/player';
  import { formatDate, formatShort } from '../../lib/format';
  import { clock } from '../player/time';
  import { uniqueKeys } from '../../lib/keys';
  import Card from './Card.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { login, onplay }: { login: string; onplay: (slug: string) => void } = $props();

  const PERIODS = [
    { value: 'day', label: '24h' },
    { value: 'week', label: '7d' },
    { value: 'month', label: '30d' },
    { value: 'all', label: 'All' },
  ] as const;
  const SORTS = [
    { value: 'views', label: 'Top' },
    { value: 'trending', label: 'Trending' },
  ] as const;

  let period = $state<ClipPeriod>('week');
  let sort = $state<ClipSort>('views');
  let clips = $state<Clip[] | null>(null);
  const clipKeys = $derived(uniqueKeys(clips ?? [], (c) => c.slug));
  let problem = $state<string | null>(null);

  $effect(() => {
    const [l, p, s] = [login, period, sort];
    let alive = true;
    clips = null;
    problem = null;
    channelIpc
      .clips(l, p, s)
      .then((list) => alive && (clips = list))
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });
</script>

<div class="page">
  <div class="filters">
    <Segmented label="Time" options={PERIODS} value={period} onchange={(v) => (period = v)} />
    <Segmented label="Order" options={SORTS} value={sort} onchange={(v) => (sort = v)} />
  </div>
  {#if problem}
    <EmptyState icon="alert" text={problem} />
  {:else if !clips}
    <Loading cards={12} kind="media" label="Loading" />
  {:else if !clips.length}
    <EmptyState icon="clapper" text="No clips in this time." />
  {:else}
    <div class="grid">
      {#each clips as c, i (clipKeys[i])}
        <Card
          image={c.thumbnail}
          title={c.title}
          sub={[c.clippedBy && `by ${c.clippedBy}`, formatDate(new Date(c.createdAt))].filter(Boolean).join(' · ')}
          corner={clock(c.duration)}
          views={`${formatShort(c.views)} ${c.views === 1 ? 'view' : 'views'}`}
          onclick={() => onplay(c.slug)}
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
