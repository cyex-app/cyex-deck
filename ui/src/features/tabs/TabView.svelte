<!-- A tab's content (U3): columns of tiles with draggable dividers. Sizes are shares,
     so the layout follows the window (3.10). Columns too narrow for their chats stack
     into one until there's room again (V102-24). Unlocked, a bar says how to move the
     panels (V102-23, V103-05). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { type Box, stackColumns, tileNeed } from '../../lib/fit';
  import { tileState } from '../player/players.svelte';
  import { drag } from './drag.svelte';
  import { dragged, layout } from './layout.svelte';
  import TileView from './TileView.svelte';
  import { tabs } from './tabs.svelte';

  interface Props {
    tab: Tab;
    popout?: boolean;
    onsignin: () => void;
    /** V102-15 Kept playing out of sight. */
    background?: boolean;
  }

  let { tab, popout = false, onsignin, background = false }: Props = $props();

  /** Smallest column or tile while dragging a divider, in px. */
  const MIN_PX = 140;

  type Sizes = { columns: number[]; rows: number[][] };
  let live = $state<Sizes | null>(null);
  const sizes = $derived<Sizes>(
    live ?? { columns: tab.columns.map((c) => c.size), rows: tab.columns.map((c) => c.tiles.map((t) => t.size)) },
  );

  let root = $state<HTMLElement>();
  let box = $state<Box>({ w: 0, h: 0 });
  /** V102-24 The columns shown one under the other, for now. */
  let stacked = $state(false);

  $effect(() => {
    if (!root) return;
    const ro = new ResizeObserver(([e]) => (box = { w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(root);
    return () => ro.disconnect();
  });

  $effect(() => {
    // Not while a tile, panel or divider is being dragged.
    if (drag.item || live) return;
    const b = box;
    if (b.w <= 0 || b.h <= 0) return;
    const columns = tab.columns.map((c) => ({
      size: c.size,
      tiles: c.tiles.map((t) => tileNeed(t.show.player || tileState(t.id).side !== null, t.show.chat && !tabs.chats.has(t.id))),
    }));
    stacked = stackColumns(b, columns, untrack(() => stacked));
  });

  const unlocked = $derived(layout.isUnlocked(tab.id) && !background);

  /** Drags the divider before column `ci`, or before tile `ri` in column `ci`. */
  function resize(e: PointerEvent, ci: number, ri?: number) {
    if (e.button !== 0 || !root) return;
    e.preventDefault();
    const vertical = ri === undefined;
    const cells = vertical
      ? [...root.querySelectorAll<HTMLElement>(':scope > .column')]
      : [...root.querySelectorAll<HTMLElement>(':scope > .column')[ci].querySelectorAll<HTMLElement>(':scope > .cell')];
    const i = vertical ? ci : ri!;
    const [a, b] = [cells[i - 1].getBoundingClientRect(), cells[i].getBoundingClientRect()];
    const start = vertical ? a.left : a.top;
    const span = vertical ? a.width + b.width : a.height + b.height;
    const base: Sizes = { columns: [...sizes.columns], rows: sizes.rows.map((r) => [...r]) };
    const list = vertical ? base.columns : base.rows[ci];
    const total = list[i - 1] + list[i];

    const move = (ev: PointerEvent) => {
      const px = Math.min(Math.max((vertical ? ev.clientX : ev.clientY) - start, MIN_PX), span - MIN_PX);
      list[i - 1] = (total * px) / span;
      list[i] = total - list[i - 1];
      live = { columns: [...base.columns], rows: base.rows.map((r) => [...r]) };
    };
    const up = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      document.body.classList.remove('resizing', 'resizing-v');
      if (live) void tabs.apply({ op: 'resize', tab: tab.id, columns: live.columns, rows: live.rows });
      live = null;
    };
    document.body.classList.add(vertical ? 'resizing' : 'resizing-v');
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
  }
</script>

<div class="tab-wrap" data-tab-view={tab.id}>
  {#if unlocked}
    <div class="unlocked">
      <span>Drag a panel by its title bar onto a side of another.</span>
      {#if dragged(tab)}<Button variant="ghost" small icon="undo" onclick={() => layout.reset(tab)}>Reset layout</Button>{/if}
      <Button variant="primary" small icon="lock" onclick={() => layout.setUnlocked(tab.id, false)}>Done</Button>
    </div>
  {/if}
  <div class="tab-view" class:stacked bind:this={root}>
    {#each tab.columns as column, ci (ci)}
      {#if ci > 0}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="divider col" onpointerdown={(e) => resize(e, ci)}></div>
      {/if}
      <div class="column" style:flex-grow={sizes.columns[ci] ?? 1}>
        {#each column.tiles as tile, ri (tile.id)}
          {#if ri > 0}
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="divider row" onpointerdown={(e) => resize(e, ci, ri)}></div>
          {/if}
          <div class="cell" style:flex-grow={stacked ? 1 : (sizes.rows[ci]?.[ri] ?? 1)}>
            <TileView {tile} {tab} {popout} {onsignin} {background} />
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .tab-wrap {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .unlocked {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px 10px;
    padding: 4px 8px 4px 12px;
    background: var(--accent-dim);
    border-bottom: 1px solid var(--accent-border);
    font-size: 0.8rem;
  }
  .unlocked span {
    flex: 1;
    min-width: 0;
  }
  .tab-view {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  /* V102-24 One under the other: the columns step aside, no dividers. */
  .stacked {
    flex-direction: column;
  }
  .stacked > .column {
    display: contents;
  }
  .stacked .divider {
    display: none;
  }
  .column {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .cell {
    flex: 1 1 0;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .stacked .cell {
    border-top: 1px solid var(--border-strong);
  }
  .stacked > .column:first-child > .cell:first-child {
    border-top: 0;
  }
  .divider {
    position: relative;
    flex: none;
    background: var(--border-strong);
  }
  /* A wider grip, only across the divider: past its ends it made the page 3 px taller. */
  .divider::after {
    content: '';
    position: absolute;
  }
  .col::after {
    inset: 0 -3px;
  }
  .row::after {
    inset: -3px 0;
  }
  .divider:hover {
    background: var(--accent-border);
  }
  .col {
    width: 1px;
    cursor: col-resize;
  }
  .row {
    height: 1px;
    cursor: row-resize;
  }
  :global(body.resizing) {
    cursor: col-resize;
  }
  :global(body.resizing-v) {
    cursor: row-resize;
  }
  :global(body.resizing *),
  :global(body.resizing-v *) {
    pointer-events: none;
  }
</style>
