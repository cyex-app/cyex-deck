<!-- V102-05 A channel's chat in its own window, opened from a `#channel` in chat. It reads
     as the account of the chat it came from, anonymous if that one was. Closing the window
     leaves the channel (unless a tab holds it); while open it counts toward the join
     limit (1.10). Route: `room/<login>[/<account id>]` (roomRoute). -->
<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import { gate } from '../../lib/gate.svelte';
  import { useHotkeys } from '../../lib/hotkeys.svelte';
  import ChatView from './ChatView.svelte';
  import { parseRoomRoute } from './roomRoute';

  let { route }: { route: string; popout?: boolean } = $props();

  const target = $derived(parseRoomRoute(route));

  useHotkeys({ 'tabs.close': () => void getCurrentWindow().close() });
</script>

{#if target}
  {#key route}
    <ChatView login={target.login} account={target.account} onsignin={() => gate.ask('helix')} />
  {/key}
{/if}
