<!-- V104-03 The frame of every pop-out window (1.51): a short title bar, and on the right
     three small icon buttons, left to right: pin (always on top), fullscreen, and close.
     Close docks the view back where it has a place in the main window (3.14, V102-16) and
     just closes where it doesn't (`#channel` chats V102-05, recordings, plugin consoles).
     No minimize. Esc leaves fullscreen. Empty parts drag the window. Usercards are pop-ups,
     not pop-outs: they keep WindowChrome with its own small x. -->
<script lang="ts">
  import { currentWindow } from '../lib/window.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    title: string;
    /** Closing puts the view back into the main window. */
    docks?: boolean;
  }

  let { title, docks = false }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.defaultPrevented || !currentWindow.fullscreen) return;
    // A player's own fullscreen goes first; the browser leaves it on this Esc.
    if (document.fullscreenElement) return;
    e.preventDefault();
    void currentWindow.setFullscreen(false);
  }
</script>

<svelte:window {onkeydown} />

<header class="frame" data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>{title}</span>
  <!-- Also where a dragged tile becomes its own tab (U3). -->
  <div class="fill" data-tauri-drag-region data-drop-bar></div>
  <div class="controls">
    <IconButton
      icon={currentWindow.pinned ? 'pinFilled' : 'pin'}
      label={currentWindow.pinned ? 'Pinned on top' : 'Pin on top'}
      hint={currentWindow.pinned ? 'Click to let other windows cover it.' : 'Keeps this window above everything else.'}
      active={currentWindow.pinned}
      size={13}
      onclick={() => currentWindow.togglePin()}
    />
    <IconButton
      icon={currentWindow.fullscreen ? 'shrink' : 'expand'}
      label={currentWindow.fullscreen ? 'Leave fullscreen' : 'Fullscreen'}
      hint={currentWindow.fullscreen ? 'Or press Esc.' : 'Fills the whole screen. Esc leaves.'}
      size={13}
      onclick={() => currentWindow.toggleFullscreen()}
    />
    {#if docks}
      <IconButton
        icon="dockBack"
        label="Dock back"
        hint="Puts it back into the main window."
        size={13}
        onclick={currentWindow.close}
      />
    {:else}
      <IconButton icon="x" label="Close" danger size={13} onclick={currentWindow.close} />
    {/if}
  </div>
</header>

<style>
  .frame {
    display: flex;
    align-items: center;
    flex: none;
    height: 28px;
    padding-left: 10px;
    background: var(--page);
    border-bottom: 1px solid var(--border);
  }
  .title {
    min-width: 0;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--muted);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .fill {
    flex: 1 0 24px;
    align-self: stretch;
  }
  .controls {
    display: flex;
    align-items: center;
    flex: none;
    gap: 1px;
    padding-right: 3px;
  }
  .controls :global(.icon-btn) {
    width: 24px;
    height: 22px;
  }
  .controls :global(.icon-btn.danger:hover) {
    background: #e81123;
    color: #fff;
  }
</style>
