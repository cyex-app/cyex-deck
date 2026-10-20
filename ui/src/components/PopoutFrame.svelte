<!-- V104-03 The frame of every pop-out window (1.51): a short title bar, and on the right
     three small icon buttons, left to right: pin (always on top), fullscreen, and close.
     Close docks the view back where it has a place in the main window (3.14, V102-16) and
     just closes where it doesn't (`#channel` chats V102-05, recordings, plugin consoles).
     No minimize. Esc leaves fullscreen. Empty parts drag the window. V105-18 Pop-ups
     (usercards, `#channel` chats) use the slim variant: the channel, pin and a small round x,
     no fullscreen; the header drags the window. -->
<script lang="ts">
  import { currentWindow } from '../lib/window.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import { tooltip } from './tooltip.svelte';

  interface Props {
    title: string;
    /** Closing puts the view back into the main window. */
    docks?: boolean;
    /** V107-R29 Where it docks back to, when that isn't the main window. */
    dockHint?: string;
    /** V105-18 `slim` for pop-ups. */
    variant?: 'large' | 'slim';
  }

  let { title, docks = false, dockHint = 'Puts it back into the main window.', variant = 'large' }: Props = $props();

  function onkeydown(e: KeyboardEvent) {
    if (variant === 'slim') return;
    if (e.key !== 'Escape' || e.defaultPrevented || !currentWindow.fullscreen) return;
    // A player's own fullscreen goes first; the browser leaves it on this Esc.
    if (document.fullscreenElement) return;
    e.preventDefault();
    void currentWindow.setFullscreen(false);
  }
</script>

<svelte:window {onkeydown} />

<header class="frame" class:slim={variant === 'slim'} data-tauri-drag-region>
  <span class="title" data-tauri-drag-region>{title}</span>
  <!-- Also where a dragged tile becomes its own tab (U3). -->
  <div class="fill" data-tauri-drag-region data-drop-bar></div>
  <div class="controls">
    <IconButton
      icon={currentWindow.pinned ? 'pinFilled' : 'pin'}
      label={currentWindow.pinned ? 'Pinned on top' : 'Pin on top'}
      hint={currentWindow.pinned ? 'Click to let other windows cover it.' : 'Keeps this window above everything else.'}
      active={currentWindow.pinned}
      size={variant === 'slim' ? 12 : 13}
      onclick={() => currentWindow.togglePin()}
    />
    {#if variant === 'large'}
      <IconButton
        icon={currentWindow.fullscreen ? 'shrink' : 'expand'}
        label={currentWindow.fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
        hint={currentWindow.fullscreen ? 'Or press Esc.' : 'Fills the whole screen. Esc leaves.'}
        size={13}
        onclick={() => currentWindow.toggleFullscreen()}
      />
    {/if}
    {#if variant === 'slim'}
      <button class="round-x" aria-label="Close" use:tooltip={{ label: 'Close' }} onclick={currentWindow.close}>
        <Icon name="x" size={10} />
      </button>
    {:else if docks}
      <IconButton
        icon="dockBack"
        label="Dock back"
        hint={dockHint}
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
    height: 24px;
  }
  .controls :global(.icon-btn.danger:hover) {
    background: var(--danger-solid);
    color: var(--on-solid);
  }
  .frame.slim {
    height: 26px;
    padding-left: 10px;
    border-bottom: 0;
  }
  .slim .controls {
    gap: 4px;
    padding-right: 6px;
  }
  .slim .controls :global(.icon-btn) {
    width: 24px;
    height: 24px;
  }
  .slim .controls :global(.icon-btn.active) {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .round-x {
    position: relative;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: 50%;
    color: var(--dim);
    transition: background var(--fast) var(--ease), color var(--fast) var(--ease);
  }
  /* 24 px to hit, 18 px to see. The 10 px x leaves a whole-pixel 4 px inset at 100 % and 150 %, so it sits
     dead centre in the ring (an 11 px one was half a pixel up and left). */
  .round-x::before {
    content: "";
    position: absolute;
    inset: -3px;
  }
  .round-x:hover {
    background: var(--danger-solid);
    color: var(--on-solid);
  }
  .round-x:active {
    transform: scale(0.94);
  }
</style>
