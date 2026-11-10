<!-- V110-A29 The pin keeps one label and a pressed state, like the pop-outs'. V110-F50 The main window's pin, minimize, maximize and close. Drawn at the title bar's right
     end (WindowChrome keeps their room free), but put last in the page, so Tab goes tab bar,
     content, then these. -->
<script lang="ts">
  import { currentWindow } from '../lib/window.svelte';
  import IconButton from './IconButton.svelte';
  import { PIN_LABEL, pinHint } from './pinHint';
</script>

<div class="controls">
  <IconButton
    chrome
    icon={currentWindow.pinned ? 'pinFilled' : 'pin'}
    label={PIN_LABEL}
    hint={pinHint(currentWindow.pinned)}
    active={currentWindow.pinned}
    toggle
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

<style>
  .controls {
    position: fixed;
    top: 0;
    right: 0;
    z-index: calc(var(--z-popover) - 2);
    display: flex;
    height: calc(var(--chrome-h) - 1px);
  }
  /* V110-F26 Narrow: smaller, so they stay on screen; WindowChrome's gap shrinks the same. */
  @media (max-width: 600px) {
    .controls :global(.icon-btn.chrome) {
      width: 34px;
    }
  }
</style>
