<!-- V1051-03 A windowed grid of fixed-size cells: only the visible rows plus two of
     overscan exist. One keyboard-driven listbox; focus stays on the scroller and
     aria-activedescendant names the cell. -->
<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  import { boxWidth, geometry, range, reveal, step } from '../lib/virtualGrid';

  interface Props<T> {
    items: readonly T[];
    key: (t: T) => string;
    label: string;
    cellWidth: number;
    cellHeight: number;
    gap?: number;
    /** The focused cell. */
    active: number;
    /** The chosen item (aria-selected). The keyboard cell is `active`, a separate thing. */
    chosen?: (t: T) => boolean;
    /** The scroller's width, bar included: a header above can end where the grid does. */
    box?: number;
    cell: Snippet<[T, { index: number; active: boolean }]>;
    /** Enter, Space, click. */
    onpick: (index: number) => void;
  }

  let { items, key, label, cellWidth, cellHeight, gap = 6, active = $bindable(), box = $bindable(0), chosen, cell, onpick }: Props<T> = $props();

  const uid = $props.id();
  const KEYS = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown']);

  let host = $state<HTMLDivElement>();
  let scroller = $state<HTMLDivElement>();
  let hostW = $state(0);
  let bar = $state(0);
  let scrollTop = $state(0);
  let width = $state(0);
  let viewH = $state(0);
  let frame = 0;

  const g = $derived(geometry(width, items.length, cellWidth, cellHeight, gap));
  // The scroller is exactly the columns wide (plus its bar): the leftover sits outside it.
  const boxW = $derived(boxWidth(g.columns, cellWidth, gap) + bar);
  $effect(() => {
    box = boxW;
  });
  const win = $derived(range(g, items.length, scrollTop, viewH));
  const shown = $derived(win.last < win.first ? [] : items.slice(win.first, win.last + 1));
  const at = (index: number) => ({
    x: (index % g.columns) * (cellWidth + gap),
    y: Math.floor(index / g.columns) * g.rowHeight,
  });

  $effect(() => {
    const el = scroller;
    const h = host;
    if (!el || !h) return;
    const read = () => {
      bar = el.offsetWidth - el.clientWidth;
      hostW = h.clientWidth;
      width = Math.max(0, hostW - bar);
      viewH = el.clientHeight;
    };
    read();
    const ro = new ResizeObserver(read);
    ro.observe(h);
    ro.observe(el);
    return () => {
      ro.disconnect();
      cancelAnimationFrame(frame);
    };
  });

  function onscroll() {
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (scroller) scrollTop = scroller.scrollTop;
    });
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.altKey || e.ctrlKey || e.metaKey) return;
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      if (active >= 0 && active < items.length) onpick(active);
      return;
    }
    if (!KEYS.has(e.key) || !items.length) return;
    e.preventDefault();
    active = step(Math.max(0, active), e.key, g, items.length, viewH);
    const to = scroller ? reveal(active, g, scroller.scrollTop, viewH) : null;
    if (scroller && to !== null) {
      scroller.scrollTop = to;
      scrollTop = to;
    }
  }

  function pick(index: number) {
    active = index;
    onpick(index);
  }
</script>

<div class="host" bind:this={host}>
<div
  class="grid"
  style:width="{boxW}px"
  role="listbox"
  tabindex="0"
  aria-label={label}
  aria-activedescendant={active >= win.first && active <= win.last ? `${uid}-${active}` : undefined}
  bind:this={scroller}
  onscroll={onscroll}
  {onkeydown}
>
  <div class="space" style:height="{g.rows * g.rowHeight}px">
    {#each shown as item, i (key(item))}
      {@const index = win.first + i}
      {@const p = at(index)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_interactive_supports_focus -->
      <div
        class="cell"
        role="option"
        id="{uid}-{index}"
        aria-selected={chosen ? chosen(item) : false}
        class:active={index === active}
        style="--x:{p.x}px;--y:{p.y}px;width:{cellWidth}px;height:{cellHeight}px"
        onclick={() => pick(index)}
      >
        {@render cell(item, { index, active: index === active })}
      </div>
    {/each}
  </div>
</div>
</div>

<style>
  .host {
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    contain: size;
  }
  .grid {
    height: 100%;
    max-width: 100%;
    overflow-y: auto;
    contain: strict;
    position: relative;
    outline: none;
  }
  .space {
    position: relative;
  }
  .cell {
    position: absolute;
    left: 0;
    top: 0;
    transform: translate(var(--x), var(--y));
    cursor: pointer;
    transition: transform var(--fast) ease-out;
  }
  .cell:active {
    transform: translate(var(--x), var(--y)) scale(0.97);
  }
  /* The ring is the text color: the accent marks the selected cell, not the focused one. */
  .grid:focus-visible .cell.active {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  @media (prefers-reduced-motion: reduce) {
    .cell {
      transition: none;
    }
    .cell:active {
      transform: translate(var(--x), var(--y));
    }
  }
</style>
