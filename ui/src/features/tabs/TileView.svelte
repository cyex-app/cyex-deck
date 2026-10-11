<!-- One channel in a tab (U3): stream, chat or both, split by a draggable divider (3.11)
     on the side set in Settings (2.1). A clip or VOD linked in a chat-only tile opens
     beside the chat (1.19, P7); while a VOD plays here the chat can replay it (P8). The
     chat can pop out on its own (1.58). In tiled tabs the header drags the tile. -->
<script lang="ts">
  import { popOut } from '../../app/panels';
  import Icon from '../../components/Icon.svelte';
  import SplitPane from '../../components/SplitPane.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { settings } from '../../lib/settings.svelte';
  import StreamSide from '../channel/StreamSide.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import FeedView from '../chat/FeedView.svelte';
  import ReplayChat from '../chat/ReplayChat.svelte';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import { openTarget, tileState } from '../player/players.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import { drag } from './drag.svelte';
  import { tileEntries } from './menus';
  import TileActions from './TileActions.svelte';
  import { chatRoute, DEFAULT_RATIO, tabs, tiles } from './tabs.svelte';

  interface Props {
    tile: Tile;
    tab: Tab;
    popout: boolean;
    onsignin: () => void;
  }

  let { tile, tab, popout, onsignin }: Props = $props();

  const tiled = $derived(tiles(tab).length > 1);
  const position = $derived(settings.v.general.chatPosition);
  const chatFirst = $derived(position === 'left' || position === 'top');
  const shared = $derived(tileState(tile.id));
  const chatPopped = $derived(tabs.chats.has(tile.id));
  const showChat = $derived(tile.show.chat && !chatPopped);
  /** 1.19, P7 Only beside a chat-only tile. */
  const side = $derived(tile.show.player ? null : shared.side);
  const replay = $derived(shared.replay ? shared.vod : null);

  const onheaderdown = $derived(
    tiled ? (e: PointerEvent) => drag.begin(e, { kind: 'tile', id: tile.id }, tile.channel.name) : undefined,
  );

  /** Drop zone shown while something that can land here is dragged over. */
  const zone = $derived.by(() => {
    const item = drag.item;
    const t = drag.target;
    if (!item || t?.kind !== 'tile' || t.id !== tile.id) return null;
    if (item.kind === 'tile') return item.id === tile.id ? null : t.zone;
    const from = tabs.tab(item.id);
    return from && from.id !== tab.id && tiles(from).length === 1 ? t.zone : null;
  });

  function setRatio(first: number) {
    const ratio = chatFirst ? first : 1 - first;
    void tabs.apply({ op: 'ratio', tile: tile.id, ratio });
  }

  /** 1.58 */
  function popChat() {
    void popOut(chatRoute(tile.id), `${tile.channel.name} chat`);
  }
</script>

{#snippet actions()}
  {#if shared.vod && !shared.replay}
    <button class="replay-btn" onclick={() => (shared.replay = true)}>Chat replay</button>
  {/if}
  <TileActions {tile} {tab} {tiled} {popout} />
{/snippet}

<!-- V101-21 A live chat's ⋯ holds the tile's entries too. -->
{#snippet chatActions()}
  {#if shared.vod && !shared.replay}
    <button class="replay-btn" onclick={() => (shared.replay = true)}>Chat replay</button>
  {/if}
  <TileActions {tile} {tab} {tiled} {popout} menu={false} />
{/snippet}

{#snippet chat()}
  {#if !showChat}
    <!-- Folded away: the chat is in its own window or turned off. -->
  {:else if tile.feed}
    <FeedView name={tile.feed} {actions} {onheaderdown} />
  {:else if replay}
    <ReplayChat
      videoId={replay.id}
      time={replay.time}
      login={tile.channel.login}
      name={tile.channel.name}
      onlive={() => (shared.replay = false)}
      {actions}
      {onheaderdown}
    />
  {:else}
    {#key `${tile.channel.login}/${tile.account}/${tile.filters.join()}`}
      <ChatView
        login={tile.channel.login}
        name={tile.channel.name}
        account={tile.account}
        readKey={tile.id}
        actions={chatActions}
        tileMenu={tiled ? () => tileEntries(tab, tile) : undefined}
        {onheaderdown}
        {onsignin}
        onlink={(t) => openTarget(tile, t)}
        filters={tile.filters}
        onfilters={(filters) => void tabs.apply({ op: 'filters', tile: tile.id, filters })}
        onpopout={tile.show.player && !popout ? popChat : undefined}
        onoverlay={() => void popOut(`overlay/${tile.id}`)}
      />
    {/key}
  {/if}
{/snippet}

{#snippet stream()}
  <StreamSide
    {tile}
    actions={showChat ? undefined : actions}
    {onheaderdown}
    onpopchat={tile.show.chat && !chatPopped && !popout ? popChat : undefined}
    onbringchat={chatPopped ? () => tabs.dockChat(tile.id) : undefined}
  />
{/snippet}

{#snippet beside()}
  <div class="beside">
    {#if side?.kind === 'clip'}
      {#key side.slug}<ClipPlayer slug={side.slug} {tile} />{/key}
    {:else if side?.kind === 'video'}
      {#key side.id}<VodPlayer id={side.id} at={side.at} {tile} account={tile.account} {shared} />{/key}
    {/if}
    <button class="close-side" aria-label="Close" onclick={() => (shared.side = null)}><Icon name="x" size={16} /></button>
  </div>
{/snippet}

<div class="tile" data-tile-id={tile.id}>
  {#if tile.show.player}
    <!-- One split either way, so the stream keeps playing when the chat pops out (1.58). -->
    <SplitPane
      direction={position === 'left' || position === 'right' ? 'row' : 'column'}
      ratio={chatFirst ? tile.ratio : 1 - tile.ratio}
      reset={chatFirst ? DEFAULT_RATIO : 1 - DEFAULT_RATIO}
      onchange={setRatio}
      first={chatFirst ? chat : stream}
      second={chatFirst ? stream : chat}
      hide={showChat ? undefined : chatFirst ? 'first' : 'second'}
    />
  {:else if showChat && side}
    <SplitPane
      direction={position === 'left' || position === 'right' ? 'row' : 'column'}
      ratio={chatFirst ? tile.ratio : 1 - tile.ratio}
      reset={chatFirst ? DEFAULT_RATIO : 1 - DEFAULT_RATIO}
      onchange={setRatio}
      first={chatFirst ? chat : beside}
      second={chatFirst ? beside : chat}
    />
  {:else if showChat}
    {@render chat()}
  {:else}
    <div class="popped">
      Chat is in its own window.
      <button onclick={() => tabs.dockChat(tile.id)}>Bring it back</button>
    </div>
  {/if}
  {#if zone}<div class="drop {zone}"></div>{/if}
</div>

<style>
  .tile {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .beside {
    position: relative;
    flex: 1;
    min-width: 0;
    display: flex;
    background: #000;
  }
  .close-side {
    position: absolute;
    top: 8px;
    right: 8px;
    z-index: 5;
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    background: rgba(0, 0, 0, 0.6);
    border: 0;
    border-radius: var(--radius);
    color: #fff;
  }
  .popped {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    color: var(--muted);
    font-size: 0.8rem;
  }
  .popped button,
  .replay-btn {
    padding: 2px 8px;
    background: var(--accent-dim);
    border: 0;
    border-radius: var(--radius);
    color: var(--accent-text);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .drop {
    position: absolute;
    z-index: 5;
    background: var(--accent-wash);
    border: 2px solid var(--accent-border);
    border-radius: var(--radius);
    pointer-events: none;
  }
  .center {
    inset: 20%;
  }
  .left {
    inset: 0 50% 0 0;
  }
  .right {
    inset: 0 0 0 50%;
  }
  .top {
    inset: 0 0 50% 0;
  }
  .bottom {
    inset: 50% 0 0 0;
  }
</style>
