<!-- V107-P4 A second click for something everyone sees (unpin, delete): a small red box
     hanging under the button it confirms, its notch on the button. Click or Enter does it;
     it goes away after 3 s, on Escape or a click elsewhere. Drawn over the page, so nothing
     moves. V107-P5 `over` hangs it over a menu item instead; the menu keeps the timer,
     the keys and the focus (menuConfirm.ts). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import { layer } from '../lib/escape';
  import { calloutAbove, calloutAt, CONFIRM_MS, expiry } from './confirmCallout';

  interface Props {
    /** The button it hangs from. */
    anchor: HTMLElement;
    /** e.g. "Click to delete". */
    label: string;
    onconfirm: () => void;
    /** Timed out, Escape or a click elsewhere. */
    ondismiss: () => void;
    /** V107-P5 Over a menu item, run by the menu: no timer, keys or focus of its own. */
    over?: boolean;
  }

  let { anchor, label, onconfirm, ondismiss, over = false }: Props = $props();

  let box = $state<HTMLButtonElement>();
  let at = $state({ left: -9999, top: -9999, notch: 0, below: true });

  /** Over everything, so the banner's clipping doesn't cut it off. */
  function toBody(node: HTMLElement) {
    document.body.appendChild(node);
    return { destroy: () => node.remove() };
  }

  function place() {
    if (!box) return;
    const r = anchor.getBoundingClientRect();
    at = over
      ? calloutAbove(r, box.offsetWidth, box.offsetHeight, innerWidth)
      : { ...calloutAt(r, box.offsetWidth, innerWidth), below: true };
  }

  // Once, when it shows: a parent redraw doesn't restart the 3 s.
  $effect(() => {
    if (box) return untrack(over ? follow : start);
  });

  /** Over a menu item: stays on it while the menu scrolls. */
  function follow() {
    place();
    addEventListener('scroll', place, true);
    addEventListener('resize', place);
    return () => {
      removeEventListener('scroll', place, true);
      removeEventListener('resize', place);
    };
  }

  function start() {
    place();
    const timer = expiry(ondismiss, CONFIRM_MS);
    const off = layer('popover', ondismiss);
    // After the menu has handed focus back to its button.
    const raf = requestAnimationFrame(() => box?.focus({ preventScroll: true }));
    const down = (e: MouseEvent) => {
      if (!box?.contains(e.target as Node)) ondismiss();
    };
    addEventListener('mousedown', down, true);
    addEventListener('resize', place);
    addEventListener('blur', ondismiss);
    return () => {
      timer.cancel();
      off();
      cancelAnimationFrame(raf);
      removeEventListener('mousedown', down, true);
      removeEventListener('resize', place);
      removeEventListener('blur', ondismiss);
    };
  }
</script>

<button
  class="callout"
  type="button"
  class:over
  class:up={!at.below}
  data-menu-confirm={over ? '' : undefined}
  tabindex={over ? -1 : undefined}
  bind:this={box}
  use:toBody
  style="left:{at.left}px;top:{at.top}px;--notch:{at.notch}px"
  onmousedown={(e) => over && e.preventDefault()}
  onclick={onconfirm}
>
  {label}
</button>

<style>
  .callout {
    position: fixed;
    z-index: var(--z-tooltip);
    height: 24px;
    padding: 0 9px;
    background: var(--danger-solid);
    border: 0;
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    color: var(--on-solid);
    font-size: var(--text-hint);
    font-weight: 600;
    white-space: nowrap;
    cursor: pointer;
  }
  /* The notch: a small triangle on the button's middle. */
  .callout::before {
    content: "";
    position: absolute;
    top: -5px;
    left: calc(var(--notch) - 5px);
    border: 5px solid transparent;
    border-top: 0;
    border-bottom-color: var(--danger-solid);
  }
  /* V107-P5 Over a menu item: above the menu, the notch at the bottom pointing down. */
  .callout.over {
    z-index: calc(var(--z-menu) + 1);
  }
  .callout.up::before {
    top: auto;
    bottom: -5px;
    border-top: 5px solid var(--danger-solid);
    border-bottom: 0;
  }
  .callout:hover {
    filter: brightness(1.08);
  }
  .callout:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
</style>
