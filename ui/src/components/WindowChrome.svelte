<!-- Title bar of the main window (3.9): mark, title or custom content, then room for pin,
     minimize, maximize and close (V110-F50: WindowControls, last in the page for Tab). Pop-outs and pop-ups have PopoutFrame (V104-03, V105-18). Empty
     areas drag the window. Grows with content that wraps (the main window's tab rows,
     V101-38); everything else stays on the first row. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { currentWindow } from '../lib/window.svelte';
  import BrandMark from './BrandMark.svelte';

  interface Props {
    title?: string;
    /** Content after the mark, e.g. the tab strip. Replaces the title. */
    start?: Snippet;
    /** Content right before the window controls. */
    end?: Snippet;
    /** V110-B7 Theatre: the bar floats over the content, out of sight until `shown`. */
    overlay?: boolean;
    shown?: boolean;
  }

  let { title = '', start, end, overlay = false, shown = false }: Props = $props();

  // V110-F101 The bar's real height (a wrapped tab row grows it), so the toasts sit under all of it.
  let height = $state(0);
  $effect(() => {
    if (!currentWindow.isMain || !height) return;
    // V110-B7 A bar over the content takes no room, so toasts sit at the top.
    document.documentElement.style.setProperty('--chrome-real-h', overlay ? '0px' : `${height}px`);
    return () => document.documentElement.style.removeProperty('--chrome-real-h');
  });
</script>

<header class="chrome" class:overlay class:shown data-tauri-drag-region bind:offsetHeight={height}>
  <div class="brand" data-tauri-drag-region><BrandMark /></div>
  {#if start}
    {@render start()}
  {:else}
    <span class="title" data-tauri-drag-region>{title}</span>
  {/if}
  <!-- Also where a dragged tile becomes its own tab (U3). -->
  <div class="fill" data-tauri-drag-region data-drop-bar></div>
  {#if end}<div class="end">{@render end()}</div>{/if}
  <!-- V110-F50 WindowControls draws the window's buttons over this gap. -->
  <div class="controls-gap" style:--buttons={currentWindow.isMain ? 4 : 3}></div>
</header>

<style>
  .chrome {
    --row: calc(var(--chrome-h) - 1px);
    display: flex;
    align-items: flex-start;
    min-height: var(--chrome-h);
    flex: none;
    background: var(--page);
    border-bottom: 1px solid var(--border);
  }
  /* V110-B7 Theatre: out of the layout, above the content, slid up out of sight; it keeps
     its place in the tab order, and focus inside it brings it down. */
  .chrome.overlay {
    position: fixed;
    inset: 0 0 auto;
    z-index: calc(var(--z-popover) - 2);
    transform: translateY(-100%);
    transition: transform var(--base) var(--ease);
  }
  .chrome.overlay.shown {
    transform: none;
    box-shadow: var(--shadow-pop);
  }
  @media (prefers-reduced-motion: reduce) {
    .chrome.overlay {
      transition: none;
    }
  }
  .brand {
    display: grid;
    place-items: center;
    flex: none;
    width: 46px;
    height: var(--row);
  }
  .brand :global(.mark) {
    transform: scale(0.72);
  }
  .title {
    align-self: center;
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* Always some room to drag by, however many tabs. */
  .fill {
    flex: 1 0 32px;
    align-self: stretch;
  }
  .end {
    display: flex;
    align-items: center;
    flex: none;
    gap: 2px;
    height: var(--row);
    padding-right: 6px;
  }
  /* V101-19 Up to the window's top edge. */
  .end :global(.icon-btn::after) {
    content: '';
    position: absolute;
    inset: -3.5px 0;
  }
  .controls-gap {
    flex: none;
    width: calc(var(--buttons) * 44px);
    height: var(--row);
  }
  /* V110-F26 Narrow (zoom 200 % on a small screen): the fixed parts shrink so Minimize,
     Maximize and Close always stay on screen. The drag strip gives way first. */
  @media (max-width: 600px) {
    .fill {
      flex-basis: 0;
    }
    .brand {
      width: 34px;
    }
    .end {
      padding-right: 2px;
    }
    .end :global(.icon-btn),
    .chrome :global(.fixed-tabs .icon-btn) {
      width: 26px;
    }
    .controls-gap {
      width: calc(var(--buttons) * 34px);
    }
  }
</style>
