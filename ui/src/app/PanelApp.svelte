<!-- A popped-out panel window (1.51): the shared pop-out frame (V104-03) plus the panel for
     its route. Usercards keep their small title bar; overlays draw their own. -->
<script lang="ts">
  import EmptyState from '../components/EmptyState.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import PopoutFrame from '../components/PopoutFrame.svelte';
  import Toasts from '../components/Toasts.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import { playerHotkeys } from '../features/player/players.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { followFullscreen } from '../lib/layers';
  import { currentWindow } from '../lib/window.svelte';
  import { panelFor, titleFor } from './panels';
  import { useRootEffects } from './root.svelte';

  let { route }: { route: string } = $props();

  const panel = $derived(panelFor(route));

  useRootEffects();
  useHotkeys({ 'window.pin': () => currentWindow.togglePin(), ...playerHotkeys });
</script>

{#if panel?.compact}
  <WindowChrome title={titleFor(route)} compact />
{:else if !panel?.bare}
  <PopoutFrame title={titleFor(route)} docks={panel?.docks} />
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
