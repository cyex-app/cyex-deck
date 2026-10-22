<!-- V1051-03 One picker: search, chips, sort and the grid. Paints and badges both use it. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import SortControl from '../../components/SortControl.svelte';
  import type { BadgeRow, PaintRow } from './cosmetics.svelte';
  import { COSMETIC_SORTS, countLabel, rememberSort, type Chip, type Query } from './filter';
  import ItemGrid from './ItemGrid.svelte';
  import SearchField from '../../components/SearchField.svelte';

  interface Props {
    kind: 'paint' | 'badge';
    query: Query;
    onquery: (q: Query) => void;
    rows: readonly (PaintRow | BadgeRow)[];
    total: number;
    selected: string | null;
    name: string;
    onpick: (id: string | null, usable: boolean) => void;
    /** Only the heading, when the pane sits under a tab already. */
    heading?: boolean;
    /** The pop-up: the grid takes what room is left, however small. */
    compact?: boolean;
  }

  let { kind, query, onquery, rows, total, selected, name, onpick, heading = true, compact = false }: Props = $props();

  const plural = $derived(kind === 'paint' ? 'paints' : 'badges');
  const CHIPS: { key: Chip; label: string }[] = [
    { key: 'animated', label: 'Animated' },
    { key: 'static', label: 'Static' },
  ];
  const chips = $derived(
    CHIPS.map((c) => ({ ...c, on: query.chips.has(c.key) })),
  );

  function change(q: Query) {
    rememberSort(kind, q);
    onquery(q);
  }

  function flip(key: string) {
    const next = new Set(query.chips);
    if (!next.delete(key as Chip)) next.add(key as Chip);
    change({ ...query, chips: next });
  }

  // V1051-03 The list can't hold more than the filter lets through.
  let box = $state(0);
  const count = $derived(countLabel(rows.length, total));
</script>

<section class="pane">
  <!-- The header ends where the grid's scrollbar does; the leftover sits outside the block. -->
  <div class="head" style:--box={box && total && rows.length ? `${box}px` : '100%'}>
  {#if heading}<h3>{kind === 'paint' ? 'Paints' : 'Badges'} · {count}</h3>{/if}
  <div class="tools">
    <div class="top">
      <div class="search">
        <SearchField placeholder="Search {plural}" value={query.text} oninput={(text) => change({ ...query, text })} />
      </div>
      <SortControl keys={COSMETIC_SORTS} sort={query.sort} label="Sort {plural}" onchange={(sort) => change({ ...query, sort })} />
    </div>
    <FilterChips {chips} onflip={flip} />
  </div>
  </div>
  <div class="body" class:compact>
    {#if !total}
      <EmptyState icon="search" text="No {plural} yet." />
    {:else if !rows.length}
      <EmptyState icon="search" text="Nothing matches." />
    {:else}
      <ItemGrid {kind} {rows} {selected} {name} label={kind === 'paint' ? 'Paints' : 'Badges'} bind:box {onpick} />
    {/if}
  </div>
</section>

<style>
  .pane {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
    min-height: 0;
  }
  .head {
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: var(--box);
    max-width: 100%;
    min-width: 0;
  }
  h3 {
    margin: 0;
    font-size: 0.85rem;
  }
  /* Search and sort on one line, the chips under them: the same for paints and badges. */
  .tools {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .top {
    display: flex;
    /* V107-R5b A narrow pop-up puts search on its own line, so its label fits. */
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .search {
    display: flex;
    flex: 1 1 150px;
    min-width: 150px;
  }
  .body {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    min-height: 240px;
  }
  .body.compact {
    min-height: 96px; /* V107-R5b fits a 313 px pop-up without a 2nd scrollbar */
  }
</style>
