<!-- Title bar of the main window (3.9): mark, title or custom content, pin,
     minimize, maximize, close. Pop-outs and pop-ups have PopoutFrame (V104-03, V105-18). Empty
     areas drag the window. Grows with content that wraps (the main window's tab rows,
     V101-38); everything else stays on the first row. -->
<script lang="ts">
  import type { Snippet } from 'svelte';

  import { currentWindow } from '../lib/window.svelte';
  import BrandMark from './BrandMark.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    title?: string;
    /** Content after the mark, e.g. the tab strip. Replaces the title. */
    start?: Snippet;
    /** Content right before the window controls. */
    end?: Snippet;
  }

  let { title = '', start, end }: Props = $props();
</script>

<header class="chrome" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region><BrandMark /></div>
  {#if start}
    {@render start()}
  {:else}
    <span class="title" data-tauri-drag-region>{title}</span>
  {/if}
  <!-- Also where a dragged tile becomes its own tab (U3). -->
  <div class="fill" data-tauri-drag-region data-drop-bar></div>
  {#if end}<div class="end">{@render end()}</div>{/if}
  <div class="controls">
    <IconButton
      chrome
      icon="pin"
      label={currentWindow.pinned ? 'Unpin' : 'Pin on top'}
      hint="Keeps this window above everything else."
      active={currentWindow.pinned}
      onclick={() => currentWindow.togglePin()}
    />
    <IconButton chrome icon="minimize" label="Minimize" onclick={currentWindow.minimize} />
    {#if currentWindow.isMain}
      <IconButton
        chrome
        icon={currentWindow.maximized ? 'restore' : 'maximize'}
        label={currentWindow.maximized ? 'Restore' : 'Maximize'}
        size={14}
        onclick={currentWindow.toggleMaximize}
      />
    {/if}
    <IconButton chrome danger icon="x" label="Close" onclick={currentWindow.close} />
  </div>
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
  .controls {
    display: flex;
    flex: none;
    height: var(--row);
  }
</style>
