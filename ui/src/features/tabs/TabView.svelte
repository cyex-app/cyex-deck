<!-- A tab's content (U3): columns of tiles with draggable dividers. Sizes are shares,
     so the layout follows the window (3.10). Columns too narrow for their chats stack
     into one until there's room again (V102-24). Unlocked, a bar says how to move the
     panels (V102-23, V103-05). F13 Every tile is absolutely placed from one list in a
     fixed order (by id), like panels since V103-05: moving a tile changes only its box, so
     no stream in the tab restarts. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { type Box, stackColumns, tileNeed } from '../../lib/fit';
  import { type Divider, dragSizes, layout as layOut, nodeAt, type Split } from '../../lib/panels';
  import { tileState } from '../player/players.svelte';
  import { drag } from './drag.svelte';
  import { gridLine, gridTree, resized } from './grid';
  import { dragged, layout } from './layout.svelte';
  import TileView from './TileView.svelte';
  import { tabs, tiles } from './tabs.svelte';

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
  /** The line between two tiles. */
  const LINE = 1;

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
    const el = root;
    if (!el) return;
    // Measured now too, so the tiles never start at no size (a player would pick its
    // quality for it).
    const r = el.getBoundingClientRect();
    box = { w: r.width, h: r.height };
    const ro = new ResizeObserver(([e]) => (box = { w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(el);
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

  /** F13 Every tile by id, in a fixed order that no move changes. */
  const byId = $derived(new Map(tiles(tab).map((t) => [t.id, t])));
  const order = $derived([...byId.keys()].sort());
  const tree = $derived(gridTree(tab.columns, sizes, stacked));
  const laid = $derived(layOut(tree, box, LINE));

  /** Drags a divider between columns, or between tiles in a column. */
  function resize(e: PointerEvent, d: Divider) {
    if (e.button !== 0 || !root || !tree || stacked) return;
    const s = nodeAt(tree, d.path) as Split | null;
    if (!s || !('dir' in s)) return;
    e.preventDefault();
    const vertical = d.dir === 'row';
    const line = gridLine(d, tab.columns.length);
    const base: Sizes = { columns: [...sizes.columns], rows: sizes.rows.map((r) => [...r]) };
    const el = root;

    const move = (ev: PointerEvent) => {
      const r = el.getBoundingClientRect();
      const at = vertical ? ev.clientX - r.left : ev.clientY - r.top;
      live = resized(base, line, dragSizes(s, d.rect, d.index, at, [MIN_PX + LINE, MIN_PX + LINE]));
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

  const px = (n: number) => `${Math.round(n * 100) / 100}px`;
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
    {#each order as id (id)}
      {@const r = laid.panels.get(id)}
      {@const tile = byId.get(id)}
      {#if tile}
        <div class="cell" style:left={px(r?.x ?? 0)} style:top={px(r?.y ?? 0)} style:width={px(r?.w ?? 0)} style:height={px(r?.h ?? 0)}>
          <TileView {tile} {tab} {popout} {onsignin} {background} />
        </div>
      {/if}
    {/each}
    {#each laid.dividers as d (`${d.path.join('.')}:${d.index}`)}
      {@const col = d.dir === 'row'}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="divider"
        class:col
        class:row={!col}
        style:left={px(col ? d.at - LINE / 2 : d.rect.x + LINE / 2)}
        style:top={px(col ? d.rect.y + LINE / 2 : d.at - LINE / 2)}
        style:width={px(col ? LINE : d.rect.w - LINE)}
        style:height={px(col ? d.rect.h - LINE : LINE)}
        onpointerdown={(e) => resize(e, d)}
      ></div>
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
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
  }
  .cell {
    position: absolute;
    display: flex;
  }
  .divider {
    position: absolute;
    background: var(--border-strong);
  }
  /* V102-24 One under the other: plain lines, nothing to drag. */
  .stacked .divider {
    pointer-events: none;
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
    cursor: col-resize;
  }
  .row {
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
