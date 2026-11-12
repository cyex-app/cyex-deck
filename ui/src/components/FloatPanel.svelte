<!-- V110-N13 A panel floating over the window: a `PanelCard` inside dragged by its title bar
     anywhere in the window and kept inside it when the window resizes; × and Escape close
     it. The card gets the drag and close through `PanelBox`. -->
<script lang="ts">
  import { onMount, type Snippet } from 'svelte';

  import { clampInto, startAt, type Pt } from './floatPlace';
  import PanelBox from './PanelBox.svelte';

  interface Props {
    /** Names the panel for screen readers. */
    label: string;
    /** How many panels opened before it, to start offset from them. */
    nth?: number;
    width?: number;
    onclose: () => void;
    children: Snippet;
  }

  let { label, nth = 0, width = 340, onclose, children }: Props = $props();

  let box = $state<HTMLElement>();
  let at = $state<Pt | null>(null);

  const view = () => ({ w: window.innerWidth, h: window.innerHeight });
  const size = () => ({ w: box?.offsetWidth ?? width, h: box?.offsetHeight ?? 0 });
  const keepIn = () => at && (at = clampInto(at, size(), view()));

  /** Focus into it: the first empty field, else the panel itself. */
  export function focus() {
    if (!box) return;
    const empty = [...box.querySelectorAll<HTMLInputElement>('input.field')].find((i) => !i.value.trim());
    (empty ?? box).focus();
  }

  onMount(() => {
    at = startAt(size(), view(), nth);
    // The card focused its first empty field; one with no form (no Affiliate) gets the panel.
    requestAnimationFrame(() => box && !box.contains(document.activeElement) && box.focus());
    // The card grows as its form or results change: still inside.
    const ro = new ResizeObserver(keepIn);
    if (box) ro.observe(box);
    window.addEventListener('resize', keepIn);
    return () => {
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
    };
    el.addEventListener('pointermove', move);
    el.addEventListener('pointerup', done);
    el.addEventListener('pointercancel', done);
  }

  const ctl = { grab, close: () => onclose(), unlocked: true };

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
  style:width="{width}px"
  style:left="{at?.x ?? 0}px"
  style:top="{at?.y ?? 0}px"
  style:visibility={at ? null : 'hidden'}
  onkeydown={key}
>
  <PanelBox {ctl}>{@render children()}</PanelBox>
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
  .float:focus {
    outline: none;
  }
</style>
