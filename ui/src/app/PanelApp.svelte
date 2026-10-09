<!-- A popped-out panel window (1.51): title bar plus the panel for its route. -->
<script lang="ts">
  import EmptyState from '../components/EmptyState.svelte';
  import Toasts from '../components/Toasts.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { currentWindow } from '../lib/window.svelte';
  import { panelFor, titleFor } from './panels';
  import { useRootEffects } from './root.svelte';

  let { route }: { route: string } = $props();

  const panel = $derived(panelFor(route));

  useRootEffects();
  useHotkeys({ 'window.pin': () => currentWindow.togglePin() });
</script>

<WindowChrome title={titleFor(route)} compact={panel?.compact} />

<main>
  {#if panel}
    <panel.component popout {route} />
  {:else}
    <EmptyState icon="x" text="This window has nothing to show." />
  {/if}
</main>

<TooltipHost />
<Toasts />

<style>
  main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
