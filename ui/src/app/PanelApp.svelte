<!-- A popped-out panel window (1.51): the shared pop-out frame (V104-03) plus the panel for
     its route. Pop-ups (usercards, `#channel` chats) get its slim look (V105-18); overlays draw their own. -->
<script lang="ts">
  import EmptyState from '../components/EmptyState.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import PopoutFrame from '../components/PopoutFrame.svelte';
  import Toasts from '../components/Toasts.svelte';
  import UnlockDialog from '../features/unlock/UnlockDialog.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import { playerHotkeys } from '../features/player/players.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import { keyTaken } from '../lib/escape';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { followFullscreen } from '../lib/layers';
  import { currentWindow } from '../lib/window.svelte';
  import { panelFor, titleFor } from './panels';
  import { useRootEffects } from './root.svelte';

  let { route }: { route: string } = $props();

  const panel = $derived(panelFor(route));

  useRootEffects();

  /** B3 Pop-ups (usercards, `#channel` chats) close on Escape once nothing else wants it:
   *  a menu or dialog took it already, and a text field keeps it for itself. */
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || panel?.frame !== 'slim' || keyTaken(e) || e.isComposing) return;
    if ((e.target as HTMLElement | null)?.closest?.('input, textarea, select, [contenteditable]')) return;
    e.preventDefault();
    void currentWindow.close();
  }
  useHotkeys({ 'window.pin': () => currentWindow.togglePin(), ...playerHotkeys });
</script>

<svelte:window {onkeydown} />

{#if panel?.frame !== 'none'}
  <PopoutFrame title={currentWindow.title ?? titleFor(route)} docks={panel?.docks} dockHint={panel?.dockHint} variant={panel?.frame === 'slim' ? 'slim' : 'large'} />
{/if}

<main>
  {#if panel}
    <panel.component popout {route} />
  {:else}
    <EmptyState icon="x" text="This window has nothing to show." />
  {/if}
</main>

<TabOverlays onopen={() => {}} />
<!-- V101-26 Kept on screen in fullscreen. -->
<div class="layers" use:followFullscreen>
  <MenuHost />
  <TooltipHost />
  <UnlockDialog />
  <Toasts />
</div>

<style>
  .layers {
    display: contents;
  }
  main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
