<!-- One channel in a tab (U3): stream, chat or both, split by a draggable divider (3.11)
     on the side set in Settings (2.1). A clip or VOD linked in a chat-only tile opens
     beside the chat (1.19, P7); while a VOD plays here the chat can replay it (P8). The
     chat can pop out on its own (1.58). In tiled tabs the header drags the tile; in an
     unlocked tab it drags the chat or stream to a side of the other (V102-23). Too small
     for both, the panels turn round until there's room again (V102-24). The shield opens
     the channel's mod view in the stream's area, the chat stepping aside or the view
     popping out when there's no room (V102-26, `fitModview`). -->
<script lang="ts">
  import { untrack } from 'svelte';

  import { popOut } from '../../app/panels';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import SplitPane from '../../components/SplitPane.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { type Box, fitModview, fitTile, type ModFit, MODVIEW, MODVIEW_SHARE, STREAM, type TileFit } from '../../lib/fit';
  import StreamSide from '../channel/StreamSide.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import FeedView from '../chat/FeedView.svelte';
  import ReplayChat from '../chat/ReplayChat.svelte';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import { openTarget, tileState } from '../player/players.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import { mods } from '../modview/moderation.svelte';
  import ModViewPanel from '../modview/ModViewPanel.svelte';
  import { modviewPlace, modviewRoute } from '../modview/where.svelte';
  import { drag, type Panel } from './drag.svelte';
  import { chatSide, layout } from './layout.svelte';
  import { layoutEntries, tileEntries } from './menus';
  import TileActions from './TileActions.svelte';
  import { chatRoute, DEFAULT_RATIO, tabs, tiles } from './tabs.svelte';
  import { sideAfterDrop } from './zones';

  interface Props {
    tile: Tile;
    tab: Tab;
    popout: boolean;
    onsignin: () => void;
    /** V102-15 Kept playing out of sight: the chat folds away meanwhile. */
    background?: boolean;
  }

  let { tile, tab, popout, onsignin, background = false }: Props = $props();

  const tiled = $derived(tiles(tab).length > 1);
  const shared = $derived(tileState(tile.id));
  const chatPopped = $derived(tabs.chats.has(tile.id));
  /** The chat shows here unless the mod view needs its room (V102-26). */
  const chatWanted = $derived(tile.show.chat && !chatPopped && !background);
  /** 1.19, P7 Only beside a chat-only tile. */
  const side = $derived(tile.show.player ? null : shared.side);
  const replay = $derived(shared.replay ? shared.vod : null);

  // V102-26 The channel's mod view in this tile, for an account that mods it (V102-21).
  const canMod = $derived(!tile.feed && mods.moderating(tile.channel.login) !== null);
  const modOpen = $derived(modviewPlace.isOpen(tile.id));
  const modWanted = $derived(canMod && modviewPlace.isIn(tile.id) && !background);
  let modFit = $state<ModFit | null>(null);
  const modShown = $derived(modWanted && modFit !== null && !modFit.out);
  const showChat = $derived(chatWanted && !(modWanted && modFit?.hideChat));
  /** The chat shares the tile with the stream, a clip or the mod view. */
  const paired = $derived(chatWanted && (tile.show.player || side !== null || modWanted));

  // V102-24 Measured, then fitted: the user's side (V102-23, else Settings 2.1), the chat
  // widened or turned round when it would be too small. Held while something's dragged.
  let root = $state<HTMLElement>();
  let box = $state<Box>({ w: 0, h: 0 });
  let fit = $state<TileFit | null>(null);
  /** A divider dragged while the panels are turned round: kept until they turn back. */
  let movedRatio = $state<number | null>(null);

  $effect(() => {
    if (!root) return;
    const ro = new ResizeObserver(([e]) => (box = { w: e.contentRect.width, h: e.contentRect.height }));
    ro.observe(root);
    return () => ro.disconnect();
  });

  $effect(() => {
    if (drag.item) return;
    const b = box;
    if (!paired) {
      fit = null;
      return;
    }
    // Out of sight (a tab kept playing) nothing changes.
    if (b.w <= 0 || b.h <= 0) return;
    // Beside a chat with no stream the mod view needs its own room (V102-26).
    const partner = tile.show.player || side !== null ? STREAM : MODVIEW;
    const next = fitTile(b, chatSide(tile), tile.ratio, untrack(() => fit?.moved ?? false), partner);
    if (!next.moved) movedRatio = null;
    fit = next;
  });

  const shownSide = $derived(fit?.side ?? chatSide(tile));
  const row = $derived(shownSide === 'left' || shownSide === 'right');
  const chatFirst = $derived(shownSide === 'left' || shownSide === 'top');
  const ratio = $derived(fit?.moved ? (movedRatio ?? fit.ratio) : (fit?.ratio ?? tile.ratio));

  // V102-26 Where the mod view goes in the stream's area, next to the chat as fitted above.
  $effect(() => {
    if (drag.item) return;
    const b = box;
    if (!modWanted) {
      modFit = null;
      return;
    }
    if (b.w <= 0 || b.h <= 0) return;
    const chat = chatWanted ? { side: shownSide, ratio } : null;
    const partner = tile.show.player ? 'stream' : side ? 'clip' : null;
    const share = tile.modviewShare ?? MODVIEW_SHARE;
    modFit = fitModview({ tile: b, chat, partner, share }, untrack(() => modFit));
  });

  /** A moment with no room before it pops out: not while a window is dragged smaller past it. */
  const OUT_AFTER_MS = 400;
  $effect(() => {
    const f = modFit;
    const id = tile.id;
    if (!f) return;
    if (!f.out) {
      untrack(() => modviewPlace.fitted(id, false));
      return;
    }
    const t = setTimeout(() => modviewPlace.fitted(id, true), OUT_AFTER_MS);
    return () => clearTimeout(t);
  });

  const modRow = $derived(modFit?.row ?? true);
  const modShare = $derived(modFit?.share ?? MODVIEW_SHARE);

  /** Kept with the tile; the default half is saved as none, so "Reset layout" has nothing to undo. */
  function setModShare(share: number) {
    void tabs.apply({ op: 'modviewShare', tile: tile.id, share: Math.abs(share - MODVIEW_SHARE) < 0.005 ? null : share });
  }

  function toggleMod() {
    modviewPlace.toggleIn(tile.id, tile.channel.login);
  }

  const unlocked = $derived(layout.isUnlocked(tab.id));

  /** In an unlocked tab a panel drags to a side of the other (V102-23); in a tiled tab the
   * header drags the tile (U3). */
  function headerDown(panel: Panel) {
    if (unlocked && paired) {
      const label = `${tile.channel.name} ${panel === 'chat' ? 'chat' : 'stream'}`;
      return (e: PointerEvent) => drag.begin(e, { kind: 'panel', tile: tile.id, panel }, label);
    }
    return tiled ? (e: PointerEvent) => drag.begin(e, { kind: 'tile', id: tile.id }, tile.channel.name) : undefined;
  }
  const onheaderdown = $derived(headerDown('chat'));
  const onstreamdown = $derived(headerDown('stream'));

  /** Drop zone shown while something that can land here is dragged over. */
  const zone = $derived.by(() => {
    const item = drag.item;
    const t = drag.target;
    if (!item || t?.kind !== 'tile' || t.id !== tile.id) return null;
    if (item.kind === 'tile') return item.id === tile.id ? null : t.zone;
    if (item.kind === 'panel') return item.tile === tile.id ? null : t.zone;
    const from = tabs.tab(item.id);
    return from && from.id !== tab.id && tiles(from).length === 1 ? t.zone : null;
  });

  /** V102-23 Where the chat lands if the panel is dropped now. */
  const landing = $derived.by(() => {
    const item = drag.item;
    const t = drag.target;
    if (item?.kind !== 'panel' || t?.kind !== 'panel' || t.tile !== tile.id) return null;
    return sideAfterDrop(item.panel, t.edge);
  });

  /** `share` is the chat's. Turned round, the divider only moves it for now. */
  function setRatio(share: number) {
    if (fit?.moved) movedRatio = share;
    else void tabs.apply({ op: 'ratio', tile: tile.id, ratio: share });
  }

  /** 1.58 */
  function popChat() {
    void popOut(chatRoute(tile.id), `${tile.channel.name} chat`);
  }
</script>

{#snippet actions()}
  {#if canMod && !showChat}
    <!-- V102-26 Where no chat carries the shield. -->
    <IconButton
      icon="shield"
      label={modOpen ? 'Close mod view' : 'Mod view'}
      hint="Mod tools for this channel."
      size={15}
      active={modOpen}
      onclick={toggleMod}
    />
  {/if}
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
        tileMenu={tiled ? () => tileEntries(tab, tile) : () => layoutEntries(tab)}
        {onheaderdown}
        {onsignin}
        onlink={(t) => openTarget(tile, t)}
        filters={tile.filters}
        onfilters={(filters) => void tabs.apply({ op: 'filters', tile: tile.id, filters })}
        onpopout={tile.show.player && !popout ? popChat : undefined}
        onoverlay={() => void popOut(`overlay/${tile.id}`)}
        onmodview={toggleMod}
        modviewOpen={modOpen}
      />
    {/key}
  {/if}
{/snippet}

{#snippet stream()}
  <StreamSide
    {tile}
    actions={showChat ? undefined : actions}
    onheaderdown={onstreamdown}
    onpopchat={tile.show.chat && !chatPopped && !popout ? popChat : undefined}
    onbringchat={chatPopped ? () => tabs.dockChat(tile.id) : undefined}
    modview={modShown ? modview : undefined}
    modSplit={{ row: modRow, share: modShare, reset: MODVIEW_SHARE, onshare: setModShare }}
  />
{/snippet}

{#snippet modview()}
  <ModViewPanel
    route={modviewRoute(tile.channel.login)}
    onpopout={() => modviewPlace.popOut(tile.id)}
    onclose={() => modviewPlace.close(tile.id)}
  />
{/snippet}

{#snippet nothing()}{/snippet}

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

{#snippet chatPanel()}
  <div class="panel" data-panel="chat">{@render chat()}</div>
{/snippet}

{#snippet streamPanel()}
  <div class="panel" data-panel="stream">{@render stream()}</div>
{/snippet}

<!-- A clip beside a chat-only tile's chat, the mod view (V102-26), or both split like a stream. -->
{#snippet besidePanel()}
  <div class="panel" data-panel="stream">
    <SplitPane
      direction={modRow ? 'row' : 'column'}
      reverse={modRow}
      ratio={1 - modShare}
      reset={1 - MODVIEW_SHARE}
      onchange={(first) => setModShare(1 - first)}
      first={beside}
      second={modShown ? modview : nothing}
      hide={!side ? 'first' : modShown ? undefined : 'second'}
    />
  </div>
{/snippet}

<div class="tile" data-tile-id={tile.id} bind:this={root}>
  {#if tile.show.player}
    <!-- One split either way, the stream always first in the page, so it keeps playing
         when the chat pops out (1.58) or changes sides (V102-23, V102-24). -->
    <SplitPane
      direction={row ? 'row' : 'column'}
      reverse={chatFirst}
      ratio={1 - ratio}
      reset={1 - DEFAULT_RATIO}
      onchange={(first) => setRatio(1 - first)}
      first={streamPanel}
      second={chatPanel}
      hide={showChat ? undefined : 'second'}
    />
  {:else if (showChat && side) || modShown}
    <SplitPane
      direction={row ? 'row' : 'column'}
      reverse={chatFirst}
      ratio={1 - ratio}
      reset={1 - DEFAULT_RATIO}
      onchange={(first) => setRatio(1 - first)}
      first={besidePanel}
      second={chatPanel}
      hide={showChat ? undefined : 'second'}
    />
  {:else if showChat}
    {@render chat()}
  {:else if background}
    <!-- V102-15 Out of sight. -->
  {:else}
    <div class="popped">
      Chat is in its own window.
      <button onclick={() => tabs.dockChat(tile.id)}>Bring it back</button>
    </div>
  {/if}
  {#if zone}<div class="drop {zone}"></div>{/if}
  {#if landing}<div class="drop chat-{landing}" style:--share={ratio}></div>{/if}
</div>

<style>
  .tile {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
  .panel {
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
  /* V102-23 Where the chat goes. */
  .chat-left {
    inset: 0 calc((1 - var(--share)) * 100%) 0 0;
  }
  .chat-right {
    inset: 0 0 0 calc((1 - var(--share)) * 100%);
  }
  .chat-top {
    inset: 0 0 calc((1 - var(--share)) * 100%) 0;
  }
  .chat-bottom {
    inset: calc((1 - var(--share)) * 100%) 0 0 0;
  }
</style>
