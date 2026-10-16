<!-- V103-05, V103-06 The one panel layout for tabs, the mod view and the stream manager.
     Every panel is placed from the tree (`lib/panels.ts`) in one flat list in a fixed
     order, so moving, closing, fitting (V102-24) or resizing never moves a panel in the
     page and a playing stream never restarts. The owner fits the tree to `box` and passes
     the result as `shown`. Unlocked, a panel drags by its title bar onto a side of
     another (or of the whole layout) and a box shows where it lands; dividers drag where
     `resizable`. Closed panels wait in a strip under the panels while it's unlocked and
     only then (`PanelTray`), taking their own room. V103-07 A `hosted` panel
     shows over a place elsewhere in the layout, e.g. its tile's player in the mod view. -->
<script lang="ts">
  import { onDestroy, type Snippet, untrack } from 'svelte';

  import { drag, panelDrops } from '../features/tabs/drag.svelte';
  import { layout as panelsState } from '../features/tabs/layout.svelte';
  import {
    type Box,
    type Divider,
    dragSizes,
    type Edge,
    layout,
    minOf,
    type Node,
    nodeAt,
    type Rect,
    setSizes,
    type Split,
  } from '../lib/panels';
  import type { PanelCtl, TrayItem } from './panel';
  import PanelBox from './PanelBox.svelte';
  import PanelTray from './PanelTray.svelte';

  interface Props {
    /** Unique in the window: drags find their layout by it. */
    key: string;
    /** The tree as fitted to `box`. */
    shown: Node | null;
    /** The size it's laid out in when that's more than the box (it scrolls). */
    size?: Box | null;
    /** Every panel that may show, in page order. */
    order: string[];
    panel: Snippet<[string, PanelCtl]>;
    unlocked?: boolean;
    resizable?: boolean;
    gap?: number;
    pad?: number;
    /** The layout's measured size. */
    box?: Box;
    /** The element panels are placed in, for `hosted` rects. */
    inner?: HTMLElement;
    /** Least sizes, for dividers. */
    min?: (id: string) => Box;
    /** A tab's tile: its panels dropped on another tile move the tile (U3). */
    tile?: string;
    /** The name a dragged panel shows. */
    label?: (id: string) => string;
    hosted?: Record<string, Rect | null>;
    tray?: TrayItem[] | null;
    /** Shown when no panel is. */
    empty?: string;
    onresize?: (path: number[], sizes: number[]) => void;
    onreset?: (path: number[], index: number) => void;
    ondrop?: (panel: string, target: string | null, edge: Edge) => void;
    onclose?: (panel: string) => void;
    onrestore?: (panel: string) => void;
  }

  let {
    key,
    shown,
    size = null,
    order,
    panel,
    unlocked = false,
    resizable = true,
    gap = 0,
    pad = 0,
    box = $bindable({ w: 0, h: 0 }),
    inner = $bindable(),
    min = () => ({ w: 0, h: 0 }),
    tile,
    label = (id) => id,
    hosted = {},
    tray = null,
    empty = '',
    onresize,
    onreset,
    ondrop,
    onclose,
    onrestore,
  }: Props = $props();

  let root = $state<HTMLElement>();

  $effect(() => {
    if (!root) return;
    const ro = new ResizeObserver(([e]) => (box = { w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(root);
    return () => ro.disconnect();
  });

  // Drops on this layout come here (`drag.svelte.ts`).
  $effect(() => {
    const k = key;
    const f = (p: string, t: string | null, e: Edge) => ondrop?.(p, t, e);
    panelDrops.set(k, f);
    return () => {
      if (panelDrops.get(k) === f) panelDrops.delete(k);
    };
  });

  /** A divider being dragged: its split's new shares, until it's let go. */
  let live = $state<{ path: number[]; sizes: number[] } | null>(null);
  const drawn = $derived(live && shown ? setSizes(shown, live.path, live.sizes) : shown);
  const room = $derived<Box>({ w: Math.max(box.w, size?.w ?? 0), h: Math.max(box.h, size?.h ?? 0) });
  const laid = $derived(layout(drawn, room, gap, pad));
  const scrolls = $derived(room.w > box.w + 0.5 || room.h > box.h + 0.5);

  // V103-07 A player shown in a slot follows it.
  $effect(() => {
    void laid;
    untrack(() => panelsState.moved++);
  });

  const rectOf = (id: string): Rect | null => hosted[id] ?? laid.panels.get(id) ?? null;
  const visible = $derived(order.filter((id) => laid.panels.has(id) || id in hosted));
  const lone = $derived(laid.panels.size <= 1);

  function ctl(id: string): PanelCtl {
    const own = laid.panels.has(id);
    return {
      unlocked,
      grab:
        unlocked && own
          ? (e) => drag.begin(e, { kind: 'panel', layout: key, panel: id, tile }, label(id))
          : undefined,
      // The last panel stays: an empty layout would read as the default one.
      close: unlocked && own && onclose && !lone ? () => onclose(id) : undefined,
    };
  }

  const dragging = (id: string) => drag.item?.kind === 'panel' && drag.item.layout === key && drag.item.panel === id;

  /** Where a panel dropped now lands: half of the panel it's on, at that edge. */
  const landing = $derived.by<Rect | null>(() => {
    const t = drag.target;
    if (t?.kind !== 'panel' || t.layout !== key) return null;
    const r = t.panel ? rectOf(t.panel) : { x: pad, y: pad, w: room.w - 2 * pad, h: room.h - 2 * pad };
    if (!r) return null;
    const half = (n: number) => n / 2;
    switch (t.edge) {
      case 'left':
        return { ...r, w: half(r.w) };
      case 'right':
        return { ...r, x: r.x + half(r.w), w: half(r.w) };
      case 'top':
        return { ...r, h: half(r.h) };
      case 'bottom':
        return { ...r, y: r.y + half(r.h), h: half(r.h) };
    }
  });

  function resizeStart(e: PointerEvent, d: Divider) {
    if (!resizable || e.button !== 0 || !shown || !inner) return;
    const s = nodeAt(shown, d.path) as Split | null;
    if (!s || !('dir' in s)) return;
    e.preventDefault();
    e.stopPropagation();
    const row = s.dir === 'row';
    const mins = s.parts.map((p) => {
      const m = minOf(p, min);
      return (row ? m.w : m.h) + gap;
    });
    const el = inner;
    // V101-14 The page goes mouse-blind only once the divider moves: from the press on, the
    // click and double-click landed on the body and never reset the divider.
    const [x0, y0] = [e.clientX, e.clientY];
    let moving = false;
    const move = (ev: PointerEvent) => {
      if (!moving) {
        if (Math.abs(ev.clientX - x0) + Math.abs(ev.clientY - y0) < 3) return;
        moving = true;
        document.body.classList.add(row ? 'panel-resize-col' : 'panel-resize-row');
      }
      const r = el.getBoundingClientRect();
      live = { path: d.path, sizes: dragSizes(s, d.rect, d.index, row ? ev.clientX - r.left : ev.clientY - r.top, mins) };
    };
    const stop = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('pointercancel', up);
      removeEventListener('blur', up);
      document.body.classList.remove('panel-resize-col', 'panel-resize-row');
      teardown = null;
    };
    // Also ends on a cancelled pointer or a lost window, and when this unmounts.
    const up = () => {
      stop();
      if (live) onresize?.(live.path, live.sizes);
      live = null;
    };
    teardown?.();
    teardown = () => {
      stop();
      live = null;
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('pointercancel', up);
    addEventListener('blur', up);
  }

  let teardown: (() => void) | null = null;
  onDestroy(() => teardown?.());

  const px = (n: number) => `${Math.round(n * 100) / 100}px`;
</script>

<div class="panels" class:dragging={drag.item?.kind === 'panel'}>
  <div class="area">
    <div class="scroll" class:scrolls data-layout={key} bind:this={root}>
      <div class="inner" style:width={scrolls ? px(room.w) : '100%'} style:height={scrolls ? px(room.h) : '100%'} bind:this={inner}>
        {#each visible as id (id)}
          {@const r = rectOf(id)}
          {@const c = ctl(id)}
          <div
            class="panel"
            class:hosted={id in hosted}
            class:away={!r}
            class:dim={dragging(id)}
            data-panel={id}
            data-panel-of={id in hosted ? undefined : key}
            style:left={px(r?.x ?? 0)}
            style:top={px(r?.y ?? 0)}
            style:width={px(r?.w ?? 0)}
            style:height={px(r?.h ?? 0)}
          >
            <PanelBox ctl={c}>{@render panel(id, c)}</PanelBox>
          </div>
        {/each}
        {#each laid.dividers as d (`${d.path.join('.')}:${d.index}`)}
          {@const row = d.dir === 'row'}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="divider"
            class:row
            class:col={!row}
            class:fixed={!resizable}
            class:line={gap === 0}
            style:left={px(row ? d.at - 4 : d.rect.x)}
            style:top={px(row ? d.rect.y : d.at - 4)}
            style:width={px(row ? 8 : d.rect.w)}
            style:height={px(row ? d.rect.h : 8)}
            onpointerdown={(e) => resizeStart(e, d)}
            ondblclick={() => resizable && onreset?.(d.path, d.index)}
          ></div>
        {/each}
        {#if landing}
          <div class="landing" style:left={px(landing.x)} style:top={px(landing.y)} style:width={px(landing.w)} style:height={px(landing.h)}></div>
        {/if}
      </div>
      {#if !visible.length && empty}
        <p class="empty">{empty}</p>
      {/if}
    </div>
  </div>
  <!-- V103-06 Hidden while locked; under the panels, never over them. -->
  {#if unlocked && tray?.length && onrestore}
    <PanelTray layout={key} items={tray} {onrestore} />
  {/if}
</div>

<style>
  .panels {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .area {
    position: relative;
    flex: 1;
    min-height: 0;
  }
  .scroll {
    position: absolute;
    inset: 0;
    overflow: hidden;
  }
  .scroll.scrolls {
    overflow: auto;
  }
  .inner {
    position: relative;
  }
  .panel {
    position: absolute;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  /* V103-07 Over the panel that gave it a place; under dividers and pop-ups. */
  .panel.hosted {
    z-index: var(--z-panel);
  }
  /* V103-07 Waiting for its slot: kept in the page, out of sight. */
  .panel.away {
    visibility: hidden;
  }
  .panel.dim {
    opacity: 0.5;
  }
  .divider {
    position: absolute;
    z-index: calc(var(--z-panel) + 1);
  }
  .divider.row {
    cursor: col-resize;
  }
  .divider.col {
    cursor: row-resize;
  }
  .divider.fixed {
    cursor: default;
    pointer-events: none;
  }
  /* The line in the middle of the 8 px grip: 1 px edge to edge, else only on hover. */
  .divider::after {
    content: '';
    position: absolute;
    transition: background var(--fast);
  }
  .divider.row::after {
    inset: 0 auto 0 calc(50% - 1px);
    width: 2px;
  }
  .divider.col::after {
    inset: calc(50% - 1px) 0 auto 0;
    height: 2px;
  }
  .divider.line.row::after {
    left: calc(50% - 0.5px);
    width: 1px;
  }
  .divider.line.col::after {
    top: calc(50% - 0.5px);
    height: 1px;
  }
  .divider.line::after {
    background: var(--border);
  }
  .divider:not(.fixed):hover::after {
    background: var(--accent-border);
  }
  .dragging .divider {
    pointer-events: none;
  }
  .landing {
    position: absolute;
    z-index: calc(var(--z-panel) + 2);
    background: var(--accent-wash);
    border: 2px solid var(--accent-border);
    border-radius: var(--radius);
    pointer-events: none;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    margin: 0;
    color: var(--muted);
    font-size: 0.85rem;
    pointer-events: none;
  }
  :global(body.panel-resize-col) {
    cursor: col-resize;
  }
  :global(body.panel-resize-row) {
    cursor: row-resize;
  }
  :global(body.panel-resize-col *),
  :global(body.panel-resize-row *) {
    pointer-events: none;
  }
  :global(body.dragging),
  :global(body.dragging *) {
    cursor: grabbing !important;
  }
</style>
