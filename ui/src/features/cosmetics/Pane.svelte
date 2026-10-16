<!-- V1051-03 One picker: search, chips, sort and the grid. Paints and badges both use it. -->
<script lang="ts">
  import EmptyState from '../../components/EmptyState.svelte';
  import FilterChips from '../../components/FilterChips.svelte';
  import Icon from '../../components/Icon.svelte';
  import Segmented from '../../components/Segmented.svelte';
    import type { BadgeRow, PaintRow } from './cosmetics.svelte';
  import { countLabel, type Chip, type Query } from './filter';
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
    /** The pop-up: the grid takes what room is left, however small. */
    compact?: boolean;
  }

  let { kind, query, onquery, rows, total, selected, name, allUsable, onpick, heading = true, compact = false }: Props = $props();

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

  // V1051-03 The list can't hold more than the filter lets through.
  const count = $derived(countLabel(rows.length, total));

  function onkey(e: KeyboardEvent) {
    if (e.key === 'Escape' && query.text) {
      e.stopPropagation();
      onquery({ ...query, text: '' });
    }
  }
</script>

<section class="pane">
  {#if heading}<h3>{kind === 'paint' ? 'Paints' : 'Badges'} · {count}</h3>{/if}
  <div class="tools">
    <div class="top">
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
        {#if query.text}
          <button class="clear" aria-label="Clear search" onclick={() => onquery({ ...query, text: '' })}>
            <Icon name="x" size={12} />
          </button>
        {/if}
      </div>
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
    <FilterChips {chips} onflip={flip} />
  </div>
  <div class="body" class:compact>
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
  /* Search and sort on one line, the chips under them: the same for paints and badges. */
  .tools {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .top {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    flex: 1 1 120px;
    min-width: 100px;
  }
  .search > :global(svg) {
    position: absolute;
    left: 9px;
    color: var(--dim);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding: 0 28px 0 29px;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    position: absolute;
    right: 1px;
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
    cursor: pointer;
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .body {
    display: grid;
    grid-template-rows: minmax(0, 1fr);
    flex: 1;
    min-height: 240px;
  }
  .body.compact {
    min-height: 120px;
  }
</style>
