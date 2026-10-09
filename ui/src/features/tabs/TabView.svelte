<!-- A tab's content (U3): columns of tiles with draggable dividers. Sizes are shares,
     so the layout follows the window (3.10). -->
<script lang="ts">
  import type { Tab } from '../../ipc/bindings/Tab';
  import TileView from './TileView.svelte';
  import { tabs } from './tabs.svelte';

  interface Props {
    tab: Tab;
    popout?: boolean;
    onsignin: () => void;
  }

  let { tab, popout = false, onsignin }: Props = $props();

  /** Smallest column or tile while dragging a divider, in px. */
  const MIN_PX = 140;

  type Sizes = { columns: number[]; rows: number[][] };
  let live = $state<Sizes | null>(null);
  const sizes = $derived<Sizes>(
    live ?? { columns: tab.columns.map((c) => c.size), rows: tab.columns.map((c) => c.tiles.map((t) => t.size)) },
  );

  let root = $state<HTMLElement>();

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

<div class="tab-view" bind:this={root}>
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
        <div class="cell" style:flex-grow={sizes.rows[ci]?.[ri] ?? 1}>
          <TileView {tile} {tab} {popout} {onsignin} />
        </div>
      {/each}
    </div>
  {/each}
</div>

<style>
  .tab-view {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
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
    min-height: 0;
    display: flex;
  }
  .divider {
    position: relative;
    flex: none;
    background: var(--border-strong);
  }
  .divider::after {
    content: '';
    position: absolute;
    inset: -3px;
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
