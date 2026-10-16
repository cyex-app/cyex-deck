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
  import Card from './Card.svelte';

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
  let problem = $state<string | null>(null);

  $effect(() => {
    const [l, p, s] = [login, period, sort];
    let alive = true;
    clips = null;
    problem = null;
    channelIpc
      .clips(l, p, s)
      .then((list) => alive && (clips = list))
      .catch((e) => alive && (problem = String(e)));
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
    <p class="load">Loading…</p>
  {:else if !clips.length}
    <EmptyState icon="clapper" text="No clips in this time." />
  {:else}
    <div class="grid">
      {#each clips as c (c.slug)}
        <Card
          image={c.thumbnail}
          title={c.title}
          sub={[c.clippedBy && `by ${c.clippedBy}`, formatDate(new Date(c.createdAt))].filter(Boolean).join(' · ')}
          corner={clock(c.duration)}
          views={`${formatShort(c.views)} views`}
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
  .load {
    margin: 0;
    padding: 8px 2px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
