<!-- V108-S1 Settings float over the window as a sheet (user, 2026/10/31): the tab behind keeps
     playing and shows a change as you make it. Escape (SettingsView), the × or a click outside
     closes it; keys behind it wait (keyHold). A small window gets it at full size. -->
<script lang="ts">
  import { fade } from 'svelte/transition';

  import IconButton from '../../components/IconButton.svelte';
  import { keyHold } from '../../lib/hotkeys.svelte';
  import SettingsView from './SettingsView.svelte';

  interface Props {
    section: string;
    onclose: () => void;
  }

  let { section = $bindable(), onclose }: Props = $props();

  let box = $state<HTMLElement>();
  const still = matchMedia('(prefers-reduced-motion: reduce)').matches;

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
  <div class="sheet" role="dialog" aria-modal="true" aria-label="Settings" tabindex="-1" bind:this={box}>
    <SettingsView bind:section {onclose} />
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
    width: min(1200px, 100%);
    height: min(880px, 100%);
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
