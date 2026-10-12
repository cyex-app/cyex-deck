<!-- V103-07 The mod view's stream panel when no tab in this window shows the channel's
     player to borrow (in its own window, or next to the tabs): a player of its own, as the
     moderating account. Its sound stays with it, not with any tab. When another window or
     tab plays the stream already, it starts paused and muted, so only that one plays on. -->
<script lang="ts">
  import type { Tile } from '../../ipc/bindings/Tile';
  import { playerIpc } from '../../ipc/player';
  import LivePlayer from '../player/LivePlayer.svelte';
  import { tabs } from '../tabs/tabs.svelte';

  let { login, account, channelId }: { login: string; account: string; channelId: string } = $props();

  /** Another player plays it now; asked once when the panel opens. */
  let elsewhere = $state<boolean | null>(null);
  $effect(() => {
    let gone = false;
    playerIpc
      .playing(login)
      .then((v) => !gone && (elsewhere = v))
      .catch(() => !gone && (elsewhere = false));
    return () => {
      gone = true;
    };
  });

  const info = $derived(tabs.live[channelId]);
  const tile = $derived<Tile>({
    id: `modview-stream/${login}`,
    channel: { id: channelId, login, name: info?.name || login },
    show: { player: true, chat: false },
    account,
    ratio: 0.28,
    size: 1,
    volume: 0.5,
    muted: elsewhere === true,
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
  {:else if elsewhere !== null}
    {#key `${login}/${account}`}
      <LivePlayer {tile} {info} held={elsewhere} />
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
