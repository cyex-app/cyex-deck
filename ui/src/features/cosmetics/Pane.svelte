<!-- V1051-03 One picker: search, chips, sort and the grid. Paints and badges both use it. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import Icon from '../../components/Icon.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { formatNumber } from '../../lib/format';
  import type { BadgeRow, PaintRow } from './cosmetics.svelte';
  import type { Chip, Query } from './filter';
  import ItemGrid from './ItemGrid.svelte';

  interface Props {
    kind: 'paint' | 'badge';
    query: Query;
    onquery: (q: Query) => void;
    rows: readonly (PaintRow | BadgeRow)[];
    total: number;
    selected: string | null;
    name: string;
    /** Hide the Usable chip when every item is usable. */
    allUsable: boolean;
    onpick: (id: string | null, usable: boolean) => void;
    /** Only the heading, when the pane sits under a tab already. */
    heading?: boolean;
  }

  let { kind, query, onquery, rows, total, selected, name, allUsable, onpick, heading = true }: Props = $props();

  const plural = $derived(kind === 'paint' ? 'paints' : 'badges');
  const CHIPS: { key: Chip; label: string }[] = [
    { key: 'animated', label: 'Animated' },
    { key: 'static', label: 'Static' },
    { key: 'usable', label: 'Usable' },
    { key: 'new', label: 'New' },
  ];
  const chips = $derived(
    CHIPS.filter((c) => c.key !== 'usable' || !allUsable).map((c) => ({ ...c, on: query.chips.has(c.key) })),
  );

  function flip(key: string) {
    const next = new Set(query.chips);
    if (!next.delete(key as Chip)) next.add(key as Chip);
    onquery({ ...query, chips: next });
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && query.text) {
      e.stopPropagation();
      onquery({ ...query, text: '' });
    }
  }
</script>

<section class="pane">
  {#if heading}<h3>{kind === 'paint' ? 'Paints' : 'Badges'} · {formatNumber(total)}</h3>{/if}
  <div class="tools">
    <div class="search">
      <Icon name="search" size={14} />
      <input
        class="field"
        type="search"
        placeholder="Search {plural}"
        aria-label="Search {plural}"
        spellcheck="false"
        value={query.text}
        oninput={(e) => onquery({ ...query, text: e.currentTarget.value })}
        onkeydown={onkey}
      />
    </div>
    <FilterChips {chips} onflip={flip} />
    <Segmented
      label="Sort"
      value={query.sort}
      options={[
        { value: 'newest', label: 'Newest' },
        { value: 'az', label: 'A–Z' },
      ]}
      onchange={(sort) => onquery({ ...query, sort })}
    />
  </div>
  <div class="body">
    {#if !total}
      <EmptyState icon="search" text="No {plural} yet." />
    {:else if !rows.length}
      <EmptyState icon="search" text="Nothing matches." />
    {:else}
      <ItemGrid {kind} {rows} {selected} {name} label={kind === 'paint' ? 'Paints' : 'Badges'} {onpick} />
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
  h3 {
    margin: 0;
    font-size: 0.85rem;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1 1 140px;
    min-width: 120px;
  }
  .search > :global(svg) {
    position: absolute;
    left: 9px;
    color: var(--dim);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding: 0 10px 0 29px;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .body {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    min-height: 240px;
  }
</style>
