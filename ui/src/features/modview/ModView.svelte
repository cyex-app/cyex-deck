<!-- 1.46 The moderator view for a channel the account mods, laid out by the user (1.53):
     V103-06 the shared panel layout (`PanelLayout`), unlocked to drag panels onto a side of
     another and to resize them, locked for neither; closed panels wait in the corner. One
     layout for every channel, in the `modview` settings. Opened from the shield (1.46),
     poppable out (1.47). The chat panel (V102-22) is a view on the room the tab already has
     open, as the moderating account; with no tab on that channel it joins while it shows.
     V102-26 Inside a tab it has a close button. V103-07 The stream panel, closed at first,
     shows the tab's own player when the view sits in that tab, else a player of its own.
     V103-15 Mod actions have their own panel next to the activity. V103-22 One chat per
     channel: its chat follows the channel's regular chat, out while that shows (in the
     tab or another window), in as a whole column while it doesn't; brought back or closed
     by hand, it stays so until "Reset layout". -->
<script lang="ts">
  import IconButton from '../../components/IconButton.svelte';
  import { grabFrom, type PanelCtl } from '../../components/panel';
  import Card from '../../components/PanelCard.svelte';
  import PanelLayout from '../../components/PanelLayout.svelte';
  import type { Board0 } from '../../ipc/bindings/Board0';
  import type { ModActionFilters } from '../../ipc/bindings/ModActionFilters';
  import { modIpc } from '../../ipc/moderation';
  import Button from '../../components/Button.svelte';
  import {
    asPatch,
    type Box,
    chatShows,
    type Edge,
    extend,
    fitTree,
    has,
    leaf,
    minOf,
    move,
    type Node,
    prune,
    remove,
    restore,
    setSizes,
    split,
    stacked,
    withChat,
  } from '../../lib/panels';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ChatView from '../chat/ChatView.svelte';
  import { chatShown } from '../chat/shown.svelte';
  import ViewerList from '../chat/ViewerList.svelte';
  import { layout } from '../tabs/layout.svelte';
  import ActivityFeed from './ActivityFeed.svelte';
  import AutomodQueue from './AutomodQueue.svelte';
  import MassActions from './MassActions.svelte';
  import ModActions from './ModActions.svelte';
  import ModStream from './ModStream.svelte';
  import { mods } from './moderation.svelte';
  import UnbanQueue from './UnbanQueue.svelte';

  interface Props {
    /** The channel login; `account` mods it. */
    login: string;
    account: string;
    /** In the main window (V102-16) and in a tab (V102-26). */
    onpopout?: () => void;
    /** In a tab (V102-26). */
    onclose?: () => void;
    /** V103-07 In a tab whose tile plays the stream: that tile, to show its player. */
    host?: string | null;
    /** V103-05 In an unlocked tab: drags the mod view by its header. */
    ongrab?: (e: PointerEvent) => void;
    /** V103-22 In a tab: the tab's own chat shows (in the tile or its own window). */
    chatNear?: boolean;
  }

  let { login, account, onpopout, onclose, host = null, ongrab, chatNear = false }: Props = $props();

  type Id = 'chat' | 'activity' | 'actions' | 'automod' | 'unban' | 'chatters' | 'mass' | 'stream';
  const INFO: Record<Id, { title: string; hint: string; min: Box; tall: number }> = {
    stream: { title: 'Stream', hint: "The channel’s stream.", min: { w: 240, h: 170 }, tall: 280 },
    chat: { title: 'Chat', hint: 'The chat, as you.', min: { w: 240, h: 200 }, tall: 440 },
    activity: { title: 'Activity', hint: 'Follows, unban requests, watched users.', min: { w: 200, h: 110 }, tall: 240 },
    actions: { title: 'Mod actions', hint: 'Bans, timeouts, deletes and more.', min: { w: 240, h: 140 }, tall: 300 },
    automod: { title: 'AutoMod queue', hint: 'Held messages to allow or deny.', min: { w: 200, h: 110 }, tall: 240 },
    unban: { title: 'Unban requests', hint: 'Who asks to be unbanned.', min: { w: 200, h: 110 }, tall: 220 },
    chatters: { title: 'Viewers', hint: "Who’s in the chat.", min: { w: 200, h: 140 }, tall: 280 },
    mass: { title: 'Mass actions', hint: 'Ban or time out many at once.', min: { w: 220, h: 140 }, tall: 260 },
  };
  /** Page order; never changes, so the chat and the stream never restart. */
  const ORDER: Id[] = ['stream', 'chat', 'activity', 'actions', 'automod', 'unban', 'chatters', 'mass'];
  /** Where every panel sits by default; a closed one comes back here. */
  const FULL: Node = split(
    'row',
    [
      split('column', [leaf('stream'), leaf('chat')], [0.4, 0.6]),
      split('column', [leaf('activity'), leaf('actions')], [0.45, 0.55]),
      split('column', [leaf('automod'), leaf('unban'), leaf('chatters'), leaf('mass')], [0.3, 0.2, 0.28, 0.22]),
    ],
    [0.34, 0.33, 0.33],
  );
  /** V103-07 Closed until asked for. */
  const START_CLOSED: string[] = ['stream'];
  /** V103-22 The chat's column in `FULL`, kept when it opens as the only chat. */
  const CHAT_SHARE = 0.34;
  const GAP = 8;
  const PAD = 8;
  const min = (id: string) => INFO[id as Id]?.min ?? { w: 200, h: 110 };

  let channelId = $state<string | null>(null);
  let board = $state<Board0>({ activity: [], actions: [], held: [], suspicious: [], older: false });
  /** What failed to load, with Retry in the header; `retry` re-runs the loads. */
  let problem = $state('');
  let retry = $state(0);

  const mv = $derived(settings.v.modview);
  const locked = $derived(mv.locked);
  /** The arrangement; before V103-06 only which widgets were removed was kept. */
  const stored = $derived<Node | null>(
    mv.layout ?? prune(FULL, (id) => !START_CLOSED.includes(id) && !mv.hidden.includes(id)),
  );

  // V103-22 One chat per channel: the chat follows the channel's regular chat, the tab's own
  // right away, other windows' as Rust tells (`chatShown`). Kept by hand, it stays so.
  $effect(() => chatShown.start());
  const kept = $derived(mv.chatKept ?? null);
  const regular = $derived(chatNear ? true : chatShown.elsewhere(login));
  const tree = $derived(withChat(stored, 'chat', chatShows(kept, regular), 'left', CHAT_SHARE));
  const closed = $derived(ORDER.filter((id) => !has(tree, id)).map((id) => ({ id, title: INFO[id].title, hint: INFO[id].hint })));

  // The id comes from an open room. V102-26 From a tab with only the stream, that's this
  // view's chat joining: asked again whenever the roles change until it's known.
  $effect(() => {
    void retry;
    if (channelId) return;
    void mods.role(account, login);
    let gone = false;
    modIpc
      .channelId(login)
      .then((id) => {
        if (gone) return;
        problem = '';
        if (id && !channelId) channelId = id;
      })
      .catch(() => !gone && (problem = 'Couldn’t load this channel.'));
    return () => (gone = true);
  });

  // Listening first, then the snapshot, so an update in between (Twitch's history coming
  // in, V103-15) isn't missed.
  $effect(() => {
    const id = channelId;
    if (!id) return;
    void retry;
    let stop: (() => void) | undefined;
    let gone = false;
    modIpc
      .onBoard(id, (b) => (board = b))
      .then((u) => {
        if (gone) return u();
        stop = u;
        return modIpc.view(id).then((b) => {
          if (gone) return;
          board = b;
          problem = '';
        });
      })
      .catch(() => !gone && (problem = 'Couldn’t load the mod activity.'));
    return () => {
      gone = true;
      stop?.();
    };
  });

  // Fitted to the room: the panels' least sizes kept; too narrow for the layout, one
  // column that scrolls (as the old grid did), which can't be moved or resized.
  let box = $state<Box>({ w: 0, h: 0 });
  const narrow = $derived(
    tree !== null && box.w > 0 && minOf(tree, (id) => ({ w: min(id).w + GAP, h: 0 })).w - GAP + 2 * PAD > box.w,
  );
  const fitted = $derived.by(() => {
    if (!tree || box.w <= 0 || box.h <= 0) return null;
    if (narrow) return stacked(tree, box.w, (id) => INFO[id as Id]?.tall ?? 240, GAP, PAD);
    return fitTree(tree, box, { min, gap: GAP, pad: PAD });
  });

  /** What shows is saved. `chat`: the chat brought back or closed by hand (V103-22). */
  function save(next: Node | null, chat?: boolean) {
    if (!next) return;
    void settings.patch('modview', { layout: asPatch(next) as never, ...(chat === undefined ? {} : { chatKept: chat }) });
  }

  function drop(panel: string, target: string | null, edge: Edge) {
    // From the corner it's brought back by hand.
    save(move(tree, panel, target, edge), panel === 'chat' && !has(tree, 'chat') ? true : undefined);
  }

  /** V103-22 The chat comes back as a whole column, as it opens by itself. */
  function bring(id: string) {
    if (id === 'chat') save(extend(tree, 'chat', 'left', CHAT_SHARE), true);
    else save(restore(tree, id, FULL));
  }

  function resize(path: number[], sizes: number[]) {
    if (tree && !narrow) save(setSizes(tree, path, sizes));
  }

  function even(path: number[]) {
    if (!tree || narrow) return;
    const s = path.reduce<Node | null>((n, i) => (n && 'dir' in n ? n.parts[i] : null), tree);
    if (s && 'dir' in s) save(setSizes(tree, path, s.parts.map(() => 1)));
  }

  function resetLayout() {
    void settings.patch('modview', { order: [], hidden: [], layout: null as never, chatKept: null as never });
    toast.success('Mod view layout reset.');
  }

  function flip(key: keyof ModActionFilters) {
    void settings.patch('modview', { modActions: { [key]: !mv.modActions[key] } });
  }

  /** V103-07 Offers the tile's player this place while the panel shows. */
  function slot(node: HTMLElement, tile: string) {
    let current = tile;
    layout.offer(current, node);
    return {
      update(next: string) {
        layout.withdraw(current, node);
        current = next;
        layout.offer(current, node);
      },
      destroy() {
        layout.withdraw(current, node);
      },
    };
  }
</script>

{#snippet panel(id: string, _ctl: PanelCtl)}
  {#if id === 'chat'}
    <Card title="Chat" flush>
      <div class="chat"><ChatView {login} {account} onmodview={null} follows /></div>
    </Card>
  {:else if id === 'activity'}
    <Card title="Activity" flush><ActivityFeed items={board.activity} suspicious={board.suspicious} /></Card>
  {:else if id === 'actions'}
    <ModActions items={board.actions} older={board.older} filters={mv.modActions} onflip={flip} {account} {login} />
  {:else if id === 'automod'}
    <Card title="AutoMod queue" flush><AutomodQueue {account} {login} held={board.held} /></Card>
  {:else if id === 'unban'}
    <Card title="Unban requests" flush><UnbanQueue {account} {login} /></Card>
  {:else if id === 'chatters'}
    <Card title="Viewers" flush><ViewerList {account} {login} /></Card>
  {:else if id === 'mass'}
    <Card title="Mass actions" flush><MassActions {account} {login} {channelId} /></Card>
  {:else if id === 'stream'}
    <Card title="Stream" flush>
      {#if host}
        <div class="slot" use:slot={host}></div>
      {:else if channelId}
        <ModStream {login} {account} {channelId} />
      {/if}
    </Card>
  {/if}
{/snippet}

<div class="modview">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={grabFrom(ongrab)} class:grab={!!ongrab}>
    <h2>Mod view · {login}</h2>
    {#if !locked}<span class="hint">Drag panels by their title.</span>{/if}
    {#if problem}
      <span class="hint bad">{problem}</span>
      <Button small variant="link" onclick={() => retry++}>Retry</Button>
    {/if}
    <div class="spacer"></div>
    {#if mv.layout || mv.hidden.length || mv.chatKept != null}
      <Button small variant="link" onclick={resetLayout}>Reset layout</Button>
    {/if}
    <IconButton
      icon={locked ? 'lock' : 'grid'}
      label={locked ? 'Unlock layout' : 'Lock layout'}
      hint={locked ? 'Move, resize and close panels.' : 'Stops panels from moving when you drag by accident.'}
      size={15}
      onclick={() => void settings.patch('modview', { locked: !locked })}
    />
    {#if onpopout}
      <IconButton icon="popout" label="Pop out" hint="Open the mod view in its own window." size={15} onclick={onpopout} />
    {/if}
    {#if onclose}
      <IconButton icon="x" label="Close mod view" size={15} onclick={onclose} />
    {/if}
  </header>

  <PanelLayout
    key="modview-{login}-{host ?? 'own'}"
    shown={fitted?.tree ?? null}
    size={fitted?.box ?? null}
    order={ORDER}
    {panel}
    unlocked={!locked}
    resizable={!locked && !narrow}
    gap={GAP}
    pad={PAD}
    bind:box
    {min}
    label={(id) => INFO[id as Id]?.title ?? id}
    tray={closed}
    empty="Every panel is closed."
    onresize={resize}
    onreset={(path) => even(path)}
    ondrop={drop}
    onclose={(id) => save(remove(tree, id), id === 'chat' ? false : undefined)}
    onrestore={bring}
  />
</div>

<style>
  .modview {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
    background: var(--page);
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }
  header.grab {
    cursor: grab;
  }
  h2 {
    margin: 0;
    font-size: 0.95rem;
    white-space: nowrap;
  }
  .hint {
    overflow: hidden;
    color: var(--muted);
    font-size: 0.75rem;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .bad {
    color: var(--red);
  }
  .spacer {
    flex: 1;
  }
  .chat {
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .slot {
    flex: 1;
    min-height: 0;
    background: #000;
  }
</style>
