<!-- Two panes with a draggable divider (3.11). `ratio` is the first pane's share, 0 to 1.
     Panes reflow with the window (3.10); the divider clamps so neither pane collapses.
     A double-click on the divider goes back to `reset` (V101-14). `reverse` shows the
     second pane first without moving it in the page, so a playing stream never restarts
     when the chat changes sides (V102-23, V102-24). -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    direction?: 'row' | 'column';
    ratio: number;
    min?: number;
    onchange: (ratio: number) => void;
    first: Snippet;
    second: Snippet;
    /** Folds one pane away without unmounting the other, e.g. a playing stream. */
    hide?: 'first' | 'second';
    /** The first pane's default share. */
    reset?: number;
    /** The second pane on the left or top. */
    reverse?: boolean;
  }

  let { direction = 'row', ratio, min = 0.15, onchange, first, second, hide, reset, reverse = false }: Props = $props();

  let container: HTMLDivElement;
  let dragging = $state(false);
  let live = $state<number | null>(null);
  const shown = $derived(live ?? ratio);

  function clamp(value: number): number {
    return Math.min(Math.max(value, min), 1 - min);
  }

  function onPointerDown(e: PointerEvent) {
    dragging = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const r = container.getBoundingClientRect();
    const at = direction === 'row' ? (e.clientX - r.left) / r.width : (e.clientY - r.top) / r.height;
    live = clamp(reverse ? 1 - at : at);
  }

  function onPointerUp() {
    if (!dragging) return;
    dragging = false;
    if (live !== null) onchange(live);
    live = null;
  }
</script>

<div class="split {direction}" class:reverse class:dragging bind:this={container}>
  <div class="pane" class:gone={hide === 'first'} class:rest={hide === 'second'} style:flex-basis="{shown * 100}%">
    {@render first()}
  </div>
  {#if !hide}
    <div
      class="divider"
      role="separator"
      aria-orientation={direction === 'row' ? 'vertical' : 'horizontal'}
      onpointerdown={onPointerDown}
      onpointermove={onPointerMove}
      onpointerup={onPointerUp}
      onpointercancel={onPointerUp}
      ondblclick={() => reset !== undefined && onchange(clamp(reset))}
    ></div>
  {/if}
  <div class="pane rest" class:gone={hide === 'second'}>{@render second()}</div>
</div>

<style>
  .split {
    display: flex;
    flex: 1;
    min-width: 0;
    min-height: 0;
  }
  .column {
    flex-direction: column;
  }
  .row.reverse {
    flex-direction: row-reverse;
  }
  .column.reverse {
    flex-direction: column-reverse;
  }
  .pane {
    display: flex;
    min-width: 0;
    min-height: 0;
    flex-grow: 0;
    flex-shrink: 0;
  }
  .rest {
    flex: 1;
  }
  .gone {
    display: none;
  }
  .divider {
    flex: none;
    background: var(--border);
    transition: background var(--fast);
  }
  .row > .divider {
    width: 1px;
    cursor: col-resize;
  }
  .column > .divider {
    height: 1px;
    cursor: row-resize;
  }
  /* Wider hit area than the visible line, above both panes (V101-14: easy to double-click). */
  .divider {
    position: relative;
    z-index: 1;
  }
  .divider::after {
    content: '';
    position: absolute;
    inset: -4px;
  }
  .divider:hover,
  .dragging .divider {
    background: var(--accent-border);
  }
  .dragging {
    user-select: none;
  }
</style>
