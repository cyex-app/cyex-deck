<!-- V1051-03 A windowed grid of fixed-size cells: only the visible rows plus two of
     overscan exist. One keyboard-driven listbox; focus stays on the scroller and
     aria-activedescendant names the cell. -->
<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  import { cellSpan, geometry, range, reveal, step } from '../lib/virtualGrid';

  interface Props<T> {
    items: readonly T[];
    key: (t: T) => string;
    label: string;
    cellWidth: number;
    cellHeight: number;
    gap?: number;
    /** The focused cell. */
    active: number;
    cell: Snippet<[T, { index: number; active: boolean }]>;
    /** Enter, Space, click. */
    onpick: (index: number) => void;
  }

  let { items, key, label, cellWidth, cellHeight, gap = 6, active = $bindable(), cell, onpick }: Props<T> = $props();

  const uid = $props.id();
  const KEYS = new Set(['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End', 'PageUp', 'PageDown']);

  let scroller = $state<HTMLDivElement>();
  let scrollTop = $state(0);
  let width = $state(0);
  let viewH = $state(0);
  let frame = 0;

  const g = $derived(geometry(width, items.length, cellWidth, cellHeight, gap));
  /** The columns stretch to fill the width; the count still comes from the asked size. */
  const cw = $derived(cellSpan(width, g.columns, cellWidth, gap));
  const win = $derived(range(g, items.length, scrollTop, viewH));
  const shown = $derived(win.last < win.first ? [] : items.slice(win.first, win.last + 1));
  const at = (index: number) => ({
    x: (index % g.columns) * (cw + gap),
    y: Math.floor(index / g.columns) * g.rowHeight,
  });

  $effect(() => {
    const el = scroller;
    if (!el) return;
    const read = () => {
      width = el.clientWidth;
      viewH = el.clientHeight;
    };
    read();
    const ro = new ResizeObserver(read);
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

<div
  class="grid"
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
        aria-selected={index === active}
        class:active={index === active}
        style="--x:{p.x}px;--y:{p.y}px;width:{cw}px;height:{cellHeight}px"
        onclick={() => pick(index)}
      >
        {@render cell(item, { index, active: index === active })}
      </div>
    {/each}
  </div>
</div>

<style>
  .grid {
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
