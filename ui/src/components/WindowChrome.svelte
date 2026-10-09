<!-- Title bar for every window (3.9): mark, title or custom content, pin, minimize,
     maximize (main only), close. Empty areas drag the window. -->
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
    /** Small windows (usercards): no minimize. */
    compact?: boolean;
  }

  let { title = '', start, end, compact = false }: Props = $props();
</script>

<header class="chrome" data-tauri-drag-region>
  <div class="brand" data-tauri-drag-region><BrandMark /></div>
  {#if start}
    {@render start()}
  {:else}
    <span class="title" data-tauri-drag-region>{title}</span>
  {/if}
  <div class="fill" data-tauri-drag-region></div>
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
    {#if !compact}
      <IconButton chrome icon="minimize" label="Minimize" onclick={currentWindow.minimize} />
    {/if}
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
    display: flex;
    align-items: center;
    height: var(--chrome-h);
    flex: none;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .brand {
    display: grid;
    place-items: center;
    width: 46px;
    height: 100%;
  }
  .brand :global(.mark) {
    transform: scale(0.72);
  }
  .title {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fill {
    flex: 1;
    align-self: stretch;
  }
  .end {
    display: flex;
    align-items: center;
    gap: 2px;
    padding-right: 6px;
  }
  .controls {
    display: flex;
    align-self: stretch;
  }
</style>
