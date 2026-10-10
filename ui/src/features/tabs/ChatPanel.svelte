<!-- 1.58 A tile's chat in its own window, so the stream can go fullscreen on another
     screen. Closing the window puts the chat back next to the stream (3.14). Clips and
     VODs linked here open with the stream in the main window. Route: `chat/<tile id>`. -->
<script lang="ts">
  import { emitTo } from '@tauri-apps/api/event';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { popOut } from '../../app/panels';
  import { windowsIpc } from '../../ipc/windows';
  import { useHotkeys } from '../../lib/hotkeys.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import { tabs, tiles } from './tabs.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  const tileId = $derived(route.split('/')[1]);
  const tile = $derived(tabs.list.flatMap(tiles).find((t) => t.id === tileId));

  $effect(() => {
    if (!tile) void getCurrentWindow().close();
  });

  $effect(() => {
    if (tile) void windowsIpc.setTitle(`${tile.channel.name} chat`);
  });

  useHotkeys({ 'tabs.close': () => void getCurrentWindow().close() });
</script>

{#if tile}
  {#key `${tile.channel.login}/${tile.account}/${tile.filters.join()}`}
    <ChatView
      login={tile.channel.login}
      name={tile.channel.name}
      account={tile.account}
      readKey={tile.id}
      filters={tile.filters}
      onfilters={(filters) => void tabs.apply({ op: 'filters', tile: tileId, filters })}
      onsignin={() => void popOut('settings')}
      onlink={(target) => void emitTo('main', 'tile:link', { tile: tileId, target })}
      onoverlay={() => void popOut(`overlay/${tileId}`)}
    />
  {/key}
{/if}
