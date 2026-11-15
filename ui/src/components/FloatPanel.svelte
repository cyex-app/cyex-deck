<!-- V110-N13 A panel floating over the window: a `PanelCard` inside dragged by its title bar
     anywhere in the window and kept inside it when the window resizes; × and Escape close
     it. The card gets the drag and close through `PanelBox`. -->
<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  import { clampInto, clampSize, recall, remember, startAt, type Pt, type Size } from './floatPlace';
  import PanelBox from './PanelBox.svelte';

  interface Props {
    /** Names the panel for screen readers. */
    label: string;
    /** How many panels opened before it, to start offset from them. */
    nth?: number;
    width?: number;
    /** V111-33 Given, the panel has this height and a grip in its corner resizes it. */
    height?: number;
    min?: Size;
    /** V111-33 Where it sat and how big it was is kept under this key until the window closes. */
    place?: string;
    /** V111-39 A thin title bar like the pop-up windows'. */
    slim?: boolean;
    onclose: () => void;
    children: Snippet;
  }

  let { label, nth = 0, width = 340, height, min = { w: 300, h: 240 }, place, slim = false, onclose, children }: Props = $props();

  let box = $state<HTMLElement>();
  let at = $state<Pt | null>(null);
  // svelte-ignore state_referenced_locally
  let dim = $state<Size>({ w: width, h: height ?? 0 });
  const sizable = $derived(height !== undefined);

  const view = () => ({ w: window.innerWidth, h: window.innerHeight });
  const size = () => ({ w: box?.offsetWidth ?? dim.w, h: box?.offsetHeight ?? 0 });
  const keep = () => place && at && remember(place, { at, size: { ...dim } });
  const keepIn = () => {
    if (!at) return;
    if (sizable) dim = clampSize(dim, min, at, view());
    at = clampInto(at, size(), view());
  };

  /** Focus into it: the first empty field, else the panel itself. */
  export function focus() {
    if (!box) return;
    const empty = [...box.querySelectorAll<HTMLInputElement>('input.field')].find((i) => !i.value.trim());
    (empty ?? box).focus();
  }

  onMount(() => {
    const was = place ? recall(place) : undefined;
    if (was) dim = was.size;
    // Sized before it is placed, so the first position knows the real box.
    at = was ? was.at : startAt({ w: dim.w, h: dim.h }, view(), nth);
    requestAnimationFrame(keepIn);
    // The card focused its first empty field; one with no form (no Affiliate) gets the panel.
    requestAnimationFrame(() => box && !box.contains(document.activeElement) && box.focus());
    // The card grows as its form or results change: still inside.
    const ro = new ResizeObserver(keepIn);
    if (box) ro.observe(box);
    window.addEventListener('resize', keepIn);
    return () => {
      keep();
      ro.disconnect();
      window.removeEventListener('resize', keepIn);
    };
  });

  function grab(e: PointerEvent) {
    if (e.button !== 0 || !at) return;
    e.preventDefault();
    const from = { x: e.clientX - at.x, y: e.clientY - at.y };
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => (at = clampInto({ x: m.clientX - from.x, y: m.clientY - from.y }, size(), view()));
    const done = () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', done);
      el.removeEventListener('pointercancel', done);
      keep();
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', done);
    el.addEventListener('pointercancel', done);
  }

  /** V111-33 The corner grip: drags the size, never under the minimum or past the window. */
  function stretch(e: PointerEvent) {
    if (e.button !== 0 || !at) return;
    e.preventDefault();
    e.stopPropagation();
    const from = { x: e.clientX - dim.w, y: e.clientY - dim.h };
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => at && (dim = clampSize({ w: m.clientX - from.x, h: m.clientY - from.y }, min, at, view()));
    const done = () => {
      el.removeEventListener('pointermove', move);
      el.removeEventListener('pointerup', done);
      el.removeEventListener('pointercancel', done);
      keep();
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', done);
    el.addEventListener('pointercancel', done);
  }

  const ctl = { grab, close: () => onclose(), unlocked: true, get slim() { return slim; } };

  /** Escape closes it, unless something inside used the key first (an open dropdown). */
  function key(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing) return;
    e.preventDefault();
    e.stopPropagation();
    onclose();
  }
</script>

<div
  class="float"
  role="dialog"
  aria-label={label}
  tabindex="-1"
  bind:this={box}
  style:width="{dim.w}px"
  style:height={sizable ? `${dim.h}px` : null}
  style:left="{at?.x ?? 0}px"
  style:top="{at?.y ?? 0}px"
  style:visibility={at ? null : 'hidden'}
  onkeydown={key}
>
  <PanelBox {ctl}>{@render children()}</PanelBox>
  {#if sizable}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="grip" onpointerdown={stretch} title="Resize"></div>
  {/if}
</div>

<style>
  .float {
    position: fixed;
    z-index: var(--z-popover);
    display: flex;
    max-width: calc(100vw - 16px);
    max-height: calc(100vh - 16px);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    animation: float-in var(--fast) var(--ease);
    transform-origin: top center;
  }
  .grip {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 14px;
    height: 14px;
    cursor: nwse-resize;
    touch-action: none;
    background: linear-gradient(135deg, transparent 55%, var(--dim) 55%, var(--dim) 62%, transparent 62%, transparent 76%, var(--dim) 76%, var(--dim) 83%, transparent 83%);
    opacity: 0.6;
  }
  .grip:hover {
    opacity: 1;
  }
  @media (prefers-reduced-motion: reduce) {
    .float {
      animation: none;
    }
  }
  .float:focus {
    outline: none;
  }
</style>
