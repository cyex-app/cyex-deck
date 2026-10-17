<!-- One channel in a tab (U3): stream, chat or both, split by a draggable divider (3.11)
     on the side set in Settings (2.1). A clip or VOD linked in a chat-only tile opens
     beside the chat (1.19, P7); while a VOD plays here the chat can replay it (P8). The
     chat can pop out on its own (1.58). In tiled tabs the header drags the tile. Too small
     for both, the panels turn round until there's room again (V102-24). The shield opens
     the channel's mod view next to the stream, the chat stepping aside or the view popping
     out when there's no room (V102-26, `fitModview`). V103-05 The panels are one panel
     layout (`PanelLayout`): in an unlocked tab any of them (stream, chat, mod view) drags
     onto a side of another, and an arrangement made by hand is kept with the tile. The
     stream never moves in the page, so it never restarts. V103-07 The mod view's stream
     panel shows this tile's player in its place. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import { popOut } from '../../app/panels';
  import IconButton from '../../components/IconButton.svelte';
  import type { PanelCtl } from '../../components/panel';
  import PanelLayout from '../../components/PanelLayout.svelte';
  import type { Tab } from '../../ipc/bindings/Tab';
  import type { Tile } from '../../ipc/bindings/Tile';
  import { type Box, fitModview, fitTile, type ModFit, MODVIEW, MODVIEW_SHARE, STREAM, type TileFit } from '../../lib/fit';
  import {
    type Edge,
    ensure,
    type Fitted,
    fitTree,
    has,
    isSplit,
    leaves,
    MARGIN,
    matchSplit,
    move,
    type Node,
    nodeAt,
    prune,
    type Rect,
    setSizes,
    setSizesByKey,
    splitKey,
  } from '../../lib/panels';
  import StreamSide from '../channel/StreamSide.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import FeedView from '../chat/FeedView.svelte';
  import KindFeed from '../feeds/KindFeed.svelte';
  import ReplayChat from '../chat/ReplayChat.svelte';
  import ClipPlayer from '../player/ClipPlayer.svelte';
  import { openTarget, tileState } from '../player/players.svelte';
  import VodPlayer from '../player/VodPlayer.svelte';
  import { mods } from '../modview/moderation.svelte';
  import ModViewPanel from '../modview/ModViewPanel.svelte';
  import { modviewPlace, modviewRoute } from '../modview/where.svelte';
  import { drag } from './drag.svelte';
  import { chatSide, layout } from './layout.svelte';
  import { layoutEntries, tileEntries } from './menus';
  import TileActions from './TileActions.svelte';
  import { chatRoute, DEFAULT_RATIO, tabs, tiles } from './tabs.svelte';
  import { chatSideOf, shareOf, TILE_PANELS, tileMin, tileTree, turnedShares } from './tileLayout';

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
  const feedTitle = $derived(tiles(tab).length === 1 && tab.alias ? tab.alias : (tile.feed ?? ''));
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

  // V103-07 The mod view's stream panel asks for this tile's player.
  const slot = $derived(tile.show.player && modWanted ? (layout.slots.get(tile.id) ?? null) : null);
  /** The stream (or a clip beside the chat) has its own place in the tile. */
  const streamIn = $derived((tile.show.player || side !== null) && !slot);
  /** The chat shares the tile with the stream, a clip or the mod view. */
  const paired = $derived(chatWanted && (streamIn || modWanted));

  /** V103-05 Arranged by hand, else from the chat's side and the mod view's fit. */
  const custom = $derived(tile.layout ?? null);

  let box = $state<Box>({ w: 0, h: 0 });
  let inner = $state<HTMLElement>();

  // V102-24 Measured, then fitted: the user's side (V102-23, else Settings 2.1), the chat
  // widened or turned round when it would be too small. Held while something's dragged.
  let fit = $state<TileFit | null>(null);
  /** A divider dragged while the panels are turned round: kept until they turn back. */
  let movedRatio = $state<number | null>(null);

  $effect(() => {
    if (drag.item || custom) return;
    const b = box;
    if (!paired) {
      fit = null;
      return;
    }
    // Out of sight (a tab kept playing) nothing changes.
    if (b.w <= 0 || b.h <= 0) return;
    // Beside a chat with no stream the mod view needs its own room (V102-26).
    const next = fitTile(b, chatSide(tile), tile.ratio, untrack(() => fit?.moved ?? false), streamIn ? STREAM : MODVIEW);
    if (!next.moved) movedRatio = null;
    fit = next;
  });

  const shownSide = $derived(fit?.side ?? chatSide(tile));
  const ratio = $derived(fit?.moved ? (movedRatio ?? fit.ratio) : (fit?.ratio ?? tile.ratio));

  // V102-26 Where the mod view goes in the stream's area, next to the chat as fitted above.
  let modFit = $state<ModFit | null>(null);
  $effect(() => {
    if (drag.item || custom) return;
    const b = box;
    if (!modWanted) {
      modFit = null;
      return;
    }
    if (b.w <= 0 || b.h <= 0) return;
    const chat = chatWanted ? { side: shownSide, ratio } : null;
    const partner = streamIn ? (tile.show.player ? 'stream' : 'clip') : null;
    const share = tile.modviewShare ?? MODVIEW_SHARE;
    modFit = fitModview({ tile: b, chat, partner, share }, untrack(() => modFit));
  });

  /** Every panel this tile has with its arrangement made by hand, in the default slots
   * for those it didn't have then. */
  function wantedIds(): string[] {
    return TILE_PANELS.filter((id) => (id === 'stream' ? streamIn : id === 'chat' ? chatWanted : modWanted));
  }

  function fullTree(b: Box): Node {
    const wide = b.w - (b.h * 16) / 9 > 0;
    return tileTree({
      stream: true,
      chat: true,
      modview: true,
      side: chatSide(tile),
      ratio: tile.ratio,
      modRow: wide,
      modShare: tile.modviewShare ?? MODVIEW_SHARE,
    })!;
  }

  // V103-05 The same rules for an arrangement made by hand: shares keep the panels' least
  // sizes, a two-panel split turns round; then the chat steps aside for the mod view, then
  // the mod view goes out into its own window. Showing the chat again takes `MARGIN` more.
  let own = $state<{ base: Node | null; fitted: Fitted | null; hideChat: boolean; out: boolean } | null>(null);
  $effect(() => {
    if (drag.item) return;
    if (!custom) {
      own = null;
      return;
    }
    const b = box;
    if (b.w <= 0 || b.h <= 0) return;
    const want = wantedIds();
    const base = ensure(
      prune(custom, (id) => want.includes(id)),
      want.filter((id) => !has(custom, id)),
      fullTree(b),
    );
    const last = untrack(() => own);
    if (!base) {
      own = { base, fitted: null, hideChat: false, out: false };
      return;
    }
    const opts = { min: tileMin, turn: true, turned: last?.fitted?.turned, turnedSizes: turnedShares };
    let f = fitTree(base, b, opts);
    let hideChat = false;
    let out = false;
    if (modWanted && has(base, 'chat')) {
      const room = last?.hideChat ? fitTree(base, { w: b.w - MARGIN, h: b.h - MARGIN }, opts).fits : f.fits;
      if (!room) {
        hideChat = true;
        f = fitTree(prune(base, (id) => id !== 'chat')!, b, opts);
      }
    }
    if (modWanted && !f.fits) {
      // No room even so: it pops out, and the tile gets its stream and chat back.
      out = true;
      hideChat = false;
      const rest = prune(base, (id) => id !== 'modview');
      f = rest ? fitTree(rest, b, opts) : f;
    }
    own = { base, fitted: f, hideChat, out };
  });

  const modReady = $derived(custom ? own !== null : modFit !== null);
  const modOut = $derived(custom ? (own?.out ?? false) : (modFit?.out ?? false));
  const hideChat = $derived(custom ? (own?.hideChat ?? false) : (modFit?.hideChat ?? false));
  const modShown = $derived(modWanted && modReady && !modOut);
  const showChat = $derived(chatWanted && !(modWanted && hideChat));

  /** A moment with no room before it pops out: not while a window is dragged smaller past it. */
  const OUT_AFTER_MS = 400;
  $effect(() => {
    const id = tile.id;
    if (!modWanted || !modReady) return;
    if (!modOut) {
      untrack(() => modviewPlace.fitted(id, false));
      return;
    }
    const t = setTimeout(() => modviewPlace.fitted(id, true), OUT_AFTER_MS);
    return () => clearTimeout(t);
  });

  const modRow = $derived(modFit?.row ?? true);
  const modShare = $derived(modFit?.share ?? MODVIEW_SHARE);

  /** V103-05 A divider dragged on a hand-made arrangement turned round to fit: kept until
   * it turns back. */
  let temp = $state<{ key: string; sizes: number[] } | null>(null);
  $effect(() => {
    if (!own?.fitted?.turned.size) temp = null;
  });

  /** What's laid out: the default fitted as before, or the hand-made one fitted. */
  const shown = $derived.by<Node | null>(() => {
    if (!custom) {
      return tileTree({ stream: streamIn, chat: showChat, modview: modShown, side: shownSide, ratio, modRow, modShare });
    }
    const t = own?.fitted?.tree ?? null;
    return t && temp ? setSizesByKey(t, temp.key, temp.sizes) : t;
  });
  const scroll = $derived(custom && own?.fitted && !own.fitted.fits ? own.fitted.box : null);

  // V103-07 This tile's player over the mod view's stream panel, following it around.
  let hostRect = $state<Rect | null>(null);
  $effect(() => {
    const el = slot;
    const base = inner;
    if (!el || !base) {
      hostRect = null;
      return;
    }
    void layout.moved;
    const measure = () => {
      const a = el.getBoundingClientRect();
      const b = base.getBoundingClientRect();
      hostRect = { x: a.left - b.left, y: a.top - b.top, w: a.width, h: a.height };
    };
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    ro.observe(base);
    return () => ro.disconnect();
  });
  const hosted = $derived<Record<string, Rect | null>>(slot ? { stream: hostRect } : {});

  /** Kept with the tile; the default half is saved as none, so "Reset layout" has nothing to undo. */
  function setModShare(share: number) {
    void tabs.apply({ op: 'modviewShare', tile: tile.id, share: Math.abs(share - MODVIEW_SHARE) < 0.005 ? null : share });
  }

  /** `share` is the chat's. Turned round, the divider only moves it for now. */
  function setRatio(share: number) {
    if (fit?.moved) movedRatio = share;
    else void tabs.apply({ op: 'ratio', tile: tile.id, ratio: share });
  }

  function resized(path: number[], sizes: number[]) {
    const s = shown ? nodeAt(shown, path) : null;
    if (!s || !isSplit(s)) return;
    if (!custom) {
      const which = shareOf(s);
      if (which?.kind === 'chat') setRatio(sizes[which.index]);
      else if (which?.kind === 'modview') setModShare(sizes[which.index]);
      return;
    }
    const at = matchSplit(custom, shown!, path);
    if (at) void tabs.apply({ op: 'layout', tile: tile.id, layout: setSizes(custom, at, sizes) });
    else temp = { key: splitKey(s), sizes };
  }

  /** V101-14 A double-click on a divider puts it back. */
  function resetSplit(path: number[]) {
    const s = shown ? nodeAt(shown, path) : null;
    if (!s || !isSplit(s)) return;
    if (!custom) {
      const which = shareOf(s);
      // Turned round to fit, the chat goes back to its fitted share, not 28 % of the height.
      if (which?.kind === 'chat' && fit?.moved) movedRatio = null;
      else if (which?.kind === 'chat') setRatio(DEFAULT_RATIO);
      else if (which?.kind === 'modview') setModShare(MODVIEW_SHARE);
      return;
    }
    resized(path, s.parts.map(() => 1 / s.parts.length));
  }

  /** V103-05 A panel dropped on a side of another. Just a stream and a chat is kept as the
   * chat's side (V102-23); anything else as an arrangement of the tile. */
  function dropped(panel: string, target: string | null, edge: Edge) {
    const base = custom
      ? (own?.base ?? custom)
      : tileTree({
          stream: streamIn,
          chat: showChat,
          modview: modShown,
          side: chatSide(tile),
          ratio: tile.ratio,
          modRow,
          modShare: tile.modviewShare ?? MODVIEW_SHARE,
        });
    const next = move(base, panel, target, edge);
    if (!next) return;
    const chat = custom ? null : chatSideOf(next);
    if (chat) void tabs.apply({ op: 'chatSide', tile: tile.id, side: chat });
    else void tabs.apply({ op: 'layout', tile: tile.id, layout: next });
  }

  function toggleMod() {
    modviewPlace.toggleIn(tile.id, tile.channel.login);
  }

  const unlocked = $derived(layout.isUnlocked(tab.id));
  const count = $derived(leaves(shown).length);
  const tileGrab = $derived(tiled ? (e: PointerEvent) => drag.begin(e, { kind: 'tile', id: tile.id }, tile.channel.name) : undefined);

  /** In an unlocked tab a panel drags to a side of another (V103-05); in a tiled tab the
   * header drags the tile (U3). */
  function grabOf(ctl: PanelCtl) {
    return ctl.grab && count > 1 ? ctl.grab : tileGrab;
  }

  /** Drop zone shown while something that can land here is dragged over. */
  const zone = $derived.by(() => {
    const item = drag.item;
    const t = drag.target;
    if (!item || t?.kind !== 'tile' || t.id !== tile.id) return null;
    if (item.kind === 'tile') return item.id === tile.id ? null : t.zone;
    if (item.kind === 'panel') return item.tile && item.tile !== tile.id ? t.zone : null;
    const from = tabs.tab(item.id);
    return from && from.id !== tab.id && tiles(from).length === 1 ? t.zone : null;
  });

  /** 1.58 */
  function popChat() {
    void popOut(chatRoute(tile.id), `${tile.channel.name} chat`);
  }

  const names: Record<string, string> = { stream: 'stream', chat: 'chat', modview: 'mod view' };
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
      toggle
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
  <TileActions {tile} {tab} {tiled} {popout} menu={false} onpopchat={tile.show.player && !chatPopped ? popChat : undefined} />
{/snippet}

{#snippet chat(onheaderdown: ((e: PointerEvent) => void) | undefined)}
  {#if tile.feed && tile.feedKind !== 'rules'}
    <KindFeed kind={tile.feedKind} name={feedTitle} {actions} {onheaderdown} />
  {:else if tile.feed}
    <FeedView name={tile.feed} title={feedTitle} {actions} {onheaderdown} />
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
        chart={!tile.show.player && tabs.live[tile.channel.id]?.stream ? tile.channel.id : undefined}
      />
    {/key}
  {/if}
{/snippet}

{#snippet beside()}
  <div class="beside">
    {#if side?.kind === 'clip'}
      {#key side.slug}<ClipPlayer slug={side.slug} {tile} onclose={() => (shared.side = null)} />{/key}
    {:else if side?.kind === 'video'}
      {#key side.id}<VodPlayer id={side.id} at={side.at} {tile} account={tile.account} {shared} onclose={() => (shared.side = null)} />{/key}
    {/if}
  </div>
{/snippet}

{#snippet panel(id: string, ctl: PanelCtl)}
  {#if id === 'stream'}
    {#if tile.show.player}
      <StreamSide
        {tile}
        actions={showChat ? undefined : actions}
        onheaderdown={grabOf(ctl)}
        onpopchat={tile.show.chat && !chatPopped && !popout ? popChat : undefined}
        onbringchat={chatPopped ? () => tabs.dockChat(tile.id) : undefined}
      />
    {:else}
      {@render beside()}
    {/if}
  {:else if id === 'chat'}
    {@render chat(grabOf(ctl))}
  {:else if id === 'modview'}
    <ModViewPanel
      route={modviewRoute(tile.channel.login)}
      onpopout={() => modviewPlace.popOut(tile.id)}
      onclose={() => modviewPlace.close(tile.id)}
      host={tile.show.player ? tile.id : null}
      ongrab={ctl.grab && count > 1 ? ctl.grab : undefined}
      chatNear={showChat || (tile.show.chat && chatPopped)}
    />
  {/if}
{/snippet}

<div class="tile" data-tile-id={tile.id}>
  <PanelLayout
    key="tile-{tile.id}"
    {shown}
    size={scroll}
    order={TILE_PANELS}
    {panel}
    {unlocked}
    bind:box
    bind:inner
    min={tileMin}
    tile={tile.id}
    label={(id) => `${tile.channel.name} ${names[id] ?? id}`}
    {hosted}
    onresize={resized}
    onreset={(path) => resetSplit(path)}
    ondrop={dropped}
  />
  {#if !shown && !slot && !background && chatPopped}
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
  .popped {
    position: absolute;
    inset: 0;
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
    white-space: nowrap;
  }
  .drop {
    position: absolute;
    z-index: var(--z-panel);
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
