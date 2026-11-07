<!-- A popped-out panel window (1.51): the shared pop-out frame (V104-03) plus the panel for
     its route. Pop-ups (usercards, `#channel` chats) get its slim look (V105-18); overlays draw their own. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import EmptyState from '../components/EmptyState.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import PopoutFrame from '../components/PopoutFrame.svelte';
  import Toasts from '../components/Toasts.svelte';
  import UnlockDialog from '../features/unlock/UnlockDialog.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import { playerHotkeys } from '../features/player/players.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import { whenResumed } from '../features/tabs/resume.svelte';
  import { keyTaken } from '../lib/escape';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { followFullscreen } from '../lib/layers';
  import { currentWindow } from '../lib/window.svelte';
  import { panelFor, titleFor } from './panels';
  import { useRootEffects } from './root.svelte';

  let { route }: { route: string } = $props();

  const panel = $derived(panelFor(route));

  useRootEffects();

  // V108-N21 After a crash, nothing joins or plays until the main window's pick is answered.
  // A tab or tile closed by the pick is gone by then, and its panel closes the window.
  // The line shows only once it is known to wait, so a normal pop-out doesn't flash it.
  // A window keeps its route.
  const waits = untrack(() => !!panelFor(route)?.waits);
  let waiting = $state<'peek' | 'held' | null>(waits ? 'peek' : null);
  if (waits) void whenResumed(() => (waiting = 'held')).then(() => (waiting = null));

  /** B3 Pop-ups (usercards, `#channel` chats) close on Escape once nothing else wants it:
   *  a menu or dialog took it already, and a text field with text clears first. */
  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Escape' || panel?.frame !== 'slim' || keyTaken(e) || e.isComposing) return;
    const field = e.target as HTMLElement | null;
    if (field?.closest?.('select, [contenteditable]')) return;
    if (field?.closest?.('input, textarea')) {
      // V109-F85 A text box clears first (the emote picker's pattern); an empty one lets the card close.
      const box = field as HTMLInputElement | HTMLTextAreaElement;
      if (box.value) {
        e.preventDefault();
        box.value = '';
        box.dispatchEvent(new Event('input', { bubbles: true }));
        return;
      }
    }
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
  {#if panel && waiting}
    {#if waiting === 'held' && panel.frame !== 'none'}<EmptyState icon="clock" text="Waiting for the main window." />{/if}
  {:else if panel}
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
