<!-- A tab in its own window (1.51). Route: `tab/<id>`. Closing the window docks the tab
     back into the main window (3.14); closing the tab closes the window. -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { popOut } from '../../app/panels';
  import { windowsIpc } from '../../ipc/windows';
  import { useHotkeys } from '../../lib/hotkeys.svelte';
  import TabView from './TabView.svelte';
  import { label, tabs } from './tabs.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  const id = $derived(route.split('/')[1]);
  const tab = $derived(tabs.tab(id));
  const title = $derived(tab ? label(tab) : '');

  $effect(() => {
    if (!tab) void getCurrentWindow().close();
  });

  $effect(() => {
    if (title) void windowsIpc.setTitle(title);
  });

  useHotkeys({ 'tabs.close': () => void getCurrentWindow().close() });
</script>

{#if tab}
  <TabView {tab} popout onsignin={() => void popOut('settings')} />
{/if}
