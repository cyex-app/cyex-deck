<!-- V103-07 The mod view's stream panel when no tab in this window shows the channel's
     player to borrow (in its own window, or next to the tabs): a player of its own, as the
     moderating account. Its sound stays with it, not with any tab. -->
<script lang="ts">
  import type { Tile } from '../../ipc/bindings/Tile';
  import LivePlayer from '../player/LivePlayer.svelte';
  import { tabs } from '../tabs/tabs.svelte';

  let { login, account, channelId }: { login: string; account: string; channelId: string } = $props();

  const info = $derived(tabs.live[channelId]);
  const tile = $derived<Tile>({
    id: `modview-stream/${login}`,
    channel: { id: channelId, login, name: info?.name || login },
    show: { player: true, chat: false },
    account,
    ratio: 0.28,
    size: 1,
    volume: 0.5,
    muted: false,
    feed: null,
    filters: [],
    chatSide: null,
    modviewShare: null,
  });
</script>

<div class="stream" data-fullscreen>
  {#if info && !info.stream}
    <!-- Known to be offline from a tab's live check: no stream to ask Twitch for. -->
    <p class="off">{tile.channel.name} is offline.</p>
  {:else}
    {#key `${login}/${account}`}
      <LivePlayer {tile} {info} />
    {/key}
  {/if}
</div>

<style>
  .stream {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    background: #000;
  }
  .off {
    margin: auto;
    color: var(--muted);
    font-size: 0.85rem;
  }
</style>
