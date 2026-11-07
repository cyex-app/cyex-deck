<!-- V108-S1 Settings float over the window as a sheet (user, 2026/10/31): the tab behind keeps
     playing and shows a change as you make it. Escape, the × or a click outside closes it; keys
     behind it wait (keyHold). A small window gets it at full size.
     V109-B15 The frame is shared: Settings, the 7TV paint and badge page and the cyex badge page.
     Settings handles its own Escape (search first); the others close on Escape here. -->
<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fade } from 'svelte/transition';

  import IconButton from './IconButton.svelte';
  import { tabbables, wrapTab } from '../lib/focusScope';
  import { keyHold } from '../lib/hotkeys.svelte';

  interface Props {
    label: string;
    onclose: () => void;
    /** The content handles Escape itself (Settings: the search first). */
    ownEscape?: boolean;
    children: Snippet;
  }

  let { label, onclose, ownEscape = false, children }: Props = $props();

  /** Escape closes unless something inside took it; a filled text field keeps it to clear itself. */
  function onkeydown(e: KeyboardEvent) {
    if (ownEscape || e.key !== 'Escape' || e.defaultPrevented || e.isComposing) return;
    const el = e.target instanceof HTMLInputElement ? e.target : null;
    if (el && el.value) return;
    e.preventDefault();
    onclose();
  }

  let box = $state<HTMLElement>();
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;

  /** Menus, dialogs and tooltips opened from the sheet live outside it, in the layers host.
   *  V108-F06 So does a confirm bubble (on the body, [data-layer]): confirming Reset is inside. */
  const inLayer = (n: EventTarget | null) => n instanceof Element && !!n.closest('.layers, [data-layer]');

  /** The rest of a press that closed the sheet goes nowhere. Outlives the sheet, which is gone by then. */
  function swallowRest() {
    const kinds = ['mousedown', 'mouseup', 'click', 'auxclick'] as const;
    const stop = (e: Event) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.type === 'click' || e.type === 'auxclick') done();
    };
    const done = () => kinds.forEach((k) => window.removeEventListener(k, stop, true));
    kinds.forEach((k) => window.addEventListener(k, stop, true));
    setTimeout(done, 1000);
  }

  // V108-S1 Tab stays in the sheet. Menus opened from it (outside it) keep their own Tab handling.
  $effect(() => {
    if (!box) return;
    const sheet = box;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Tab' || e.defaultPrevented) return;
      const at = document.activeElement;
      if (inLayer(at) || inLayer(e.target)) return;
      const stops = tabbables(sheet);
      const next = wrapTab(stops.indexOf(at as HTMLElement), stops.length, e.shiftKey);
      e.preventDefault();
      (stops[next] ?? sheet).focus();
    };
    // The title bar stays clickable (a tab click closes the sheet) but has no tab menus behind it.
    const onMenu = (e: MouseEvent) => {
      if (sheet.contains(e.target as Node) || inLayer(e.target)) return;
      e.preventDefault();
      e.stopPropagation();
    };
    // A click outside (the title bar's tabs and buttons) only closes the sheet; a second click
    // acts (user, 2026/11/01). Dragging the window and its own buttons still work.
    const onDown = (e: PointerEvent) => {
      const at = e.target instanceof Element ? e.target : null;
      if (!at || sheet.parentElement?.contains(at) || inLayer(at) || at.closest('.controls')) return;
      onclose();
      if (!at.closest('button, a, input, [role="tab"], [role="button"], [tabindex]')) return;
      e.preventDefault();
      e.stopPropagation();
      swallowRest();
    };
    window.addEventListener('keydown', onKey, true);
    window.addEventListener('contextmenu', onMenu, true);
    window.addEventListener('pointerdown', onDown, true);
    return () => {
      window.removeEventListener('keydown', onKey, true);
      window.removeEventListener('contextmenu', onMenu, true);
      window.removeEventListener('pointerdown', onDown, true);
    };
  });

  $effect(() => {
    keyHold.sheet = true;
    const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    box?.focus({ preventScroll: true });
    return () => {
      keyHold.sheet = false;
      // Back where you were typing, once what's behind takes focus again.
      requestAnimationFrame(() => before?.isConnected && before.focus({ preventScroll: true }));
    };
  });
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()} out:fade|global={{ duration: still ? 0 : 100 }}>
  <div class="sheet" role="dialog" aria-modal="true" aria-label={label} tabindex="-1" bind:this={box} {onkeydown}>
    {@render children()}
    <span class="close"><IconButton icon="x" label="Close" hint="Esc" size={15} onclick={onclose} /></span>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: var(--chrome-h) 0 0;
    /* Over the tabs, under menus, dialogs and tooltips it opens. */
    z-index: calc(var(--z-popover) - 10);
    display: grid;
    grid-template: minmax(0, 1fr) / minmax(0, 1fr);
    place-items: center;
    padding: 28px;
    background: var(--overlay);
    animation: fade var(--fast) ease-out;
  }
  .sheet {
    position: relative;
    display: flex;
    /* V109-F75 Grows with the window; never past it (a small window gets all of it, below). */
    width: min(clamp(720px, 72vw, 1800px), 100%);
    height: min(clamp(480px, 84vh, 1300px), 100%);
    background: var(--bg);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
    animation: pop var(--base) var(--ease);
  }
  .sheet:focus {
    outline: none;
  }
  .close {
    position: absolute;
    top: 6px;
    /* Clear of the page's scrollbar (11px). */
    right: 17px;
  }
  /* A small window: the whole window, as the page was. */
  @media (max-width: 900px), (max-height: 560px) {
    .backdrop {
      padding: 0;
    }
    .sheet {
      width: 100%;
      height: 100%;
      border: 0;
      border-radius: 0;
      box-shadow: none;
    }
  }
  @keyframes fade {
    from {
      opacity: 0;
    }
  }
  @keyframes pop {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .backdrop,
    .sheet {
      animation: none;
    }
  }
</style>
