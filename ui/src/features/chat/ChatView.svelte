<!-- A channel's chat (1.8, 1.9): header with room modes and status, pinned message,
     messages, message box. The header carries the tile's buttons and drag handle. -->
<script module lang="ts">
  const views = new Set<HTMLElement>();
  let lastUsed: HTMLElement | null = null;

  /** The chat a chat hotkey is for: the focused one, else the last one used, else the first on screen. */
  function hotkeyTarget(): HTMLElement | null {
    const shown = [...views].filter((v) => v.checkVisibility());
    const focused = shown.find((v) => v.contains(document.activeElement));
    return focused ?? (lastUsed && shown.includes(lastUsed) ? lastUsed : (shown[0] ?? null));
  }
</script>

<script lang="ts">
  import { onDestroy, type Snippet } from 'svelte';

  import IconButton from '../../components/IconButton.svelte';
  import { openMenu, type MenuEntry } from '../../components/menu.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { ConnStatus } from '../../ipc/bindings/ConnStatus';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { chatIpc } from '../../ipc/chat';
  import { chatxIpc } from '../../ipc/chatx';
  import { errorText } from '../../lib/errors';
  import { formatDuration } from '../../lib/format';
  import { lockFor } from '../../lib/gate';
  import { bindingOf, useHotkeys } from '../../lib/hotkeys.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { catalogs } from './catalog.svelte';
  import ChatInput from './ChatInput.svelte';
  import CosmeticsPopup from '../cosmetics/CosmeticsPopup.svelte';
  import EventsButton from './EventsButton.svelte';
  import { runEntry, toItems, userMenus } from './menus.svelte';
  import MessageList from './MessageList.svelte';
  import PinnedBanner from './PinnedBanner.svelte';
  import { ChatRoom } from './room.svelte';
  import SearchPopup from './SearchPopup.svelte';
  import ViewerList from './ViewerList.svelte';
  import { votes } from './votes.svelte';
  import VotesBanner from './VotesBanner.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { mods } from '../modview/moderation.svelte';
  import { modviewPlace } from '../modview/where.svelte';
  import { pluginItems } from '../plugins/pluginMenus';
  import { plugins } from '../plugins/plugins.svelte';
  import { pluginsIpc } from '../../ipc/plugins';
  import ResubBanner from '../subs/ResubBanner.svelte';
  import SubDialog from '../subs/SubDialog.svelte';

  interface Props {
    login: string;
    /** Display name for the header. */
    name?: string;
    /** User id of the reading account; `null` reads anonymously. */
    account: string | null;
    /** Keeps the last-read line (C5) across tab switches. */
    readKey?: string;
    /** Buttons at the end of the header. */
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
    onsignin?: () => void;
    /** Clips and VODs linked in chat open here (1.19, 1.20, P7). */
    onlink?: (target: LinkTarget) => void;
    /** F3 Filter ids this chat shows messages through, and where a change goes. */
    filters?: string[];
    onfilters?: (ids: string[]) => void;
    /** `/popout` (1.58). */
    onpopout?: () => void;
    /** U7 Opens this chat as an overlay. */
    onoverlay?: () => void;
    /** V101-21 A tiled tab's entries for this tile, after the chat's in its one menu. */
    tileMenu?: () => MenuEntry[];
    /** V102-26 The shield: in a tab it opens the mod view in the tab, open (`modviewOpen`)
     * it closes it. `null` leaves it out (the mod view's own chat). By default it opens the
     * mod view next to the tabs or in its own window (V102-16). */
    onmodview?: (() => void) | null;
    modviewOpen?: boolean;
    /** V103-22 The mod view's or the stream manager's own chat: it follows the channel's
     * regular chats and doesn't count as one. */
    follows?: boolean;
  }

  let {
    login,
    name,
    account,
    readKey,
    actions,
    onheaderdown,
    onsignin = () => {},
    onlink,
    filters = [],
    onfilters,
    onpopout,
    onoverlay,
    tileMenu,
    onmodview = () => modviewPlace.open(login),
    modviewOpen = false,
    follows = false,
  }: Props = $props();

  // A chat view shows one channel for its whole life; a different one is a new view.
  // svelte-ignore state_referenced_locally
  const room = new ChatRoom(login);
  // A filter change keys a new view, so the list it opened with stays right.
  // svelte-ignore state_referenced_locally
  void room.open(account, filters, follows).then(() => {
    if (room.view) chatxIpc.recording(room.view).then((r) => (recording = r)).catch(() => {});
  });
  void catalogs.loadGlobal();
  // svelte-ignore state_referenced_locally
  void catalogs.loadChannel(login);
  // svelte-ignore state_referenced_locally
  votes.watch(login);
  onDestroy(() => room.close());

  let el = $state<HTMLElement>();
  /** V101-35 The viewer list shows instead of the messages. */
  let viewers = $state(false);
  let input = $state<ChatInput>();
  let replyTo = $state<ChatMessage | null>(null);
  let search = $state<{ query: string; all: boolean } | null>(null);
  let recording = $state(false);
  /** 1.28 The subscribe dialog is open. */
  let subscribing = $state(false);

  $effect(() => {
    if (!el) return;
    const me = el;
    views.add(me);
    return () => {
      views.delete(me);
      if (lastUsed === me) lastUsed = null;
    };
  });

  $effect(() => {
    let stop: (() => void) | undefined;
    let gone = false;
    void chatxIpc
      .onRecording((e) => {
        if (e.channel === login) recording = e.recording;
      })
      .then((u) => (gone ? u() : (stop = u)));
    return () => {
      gone = true;
      stop?.();
    };
  });

  $effect(() => userMenus.track());

  const mine = (run: () => void) => () => (hotkeyTarget() === el ? run() : false);
  useHotkeys({
    'chat.search': mine(() => (search = { query: search?.query ?? '', all: false })),
    'chat.searchAll': mine(() => (search = { query: search?.query ?? '', all: true })),
    'chat.similar': mine(() => {
      const hide = !settings.v.highlights.similar.hide;
      void settings.patch('highlights', { similar: { hide } });
      toast.info(hide ? 'Near-duplicates hidden' : 'Near-duplicates shown');
    }),
  });

  // V1051-03 /cosmetics
  let cosmeticsOpen = $state(false);
  function closeCosmetics() {
    cosmeticsOpen = false;
    input?.focus();
  }

  function onaction(a: UiAction) {
    if (a.type === 'search') search = { query: a.query, all: false };
    else if (a.type === 'cosmetics') cosmeticsOpen = true;
    else if (a.type === 'popout') {
      if (onpopout) onpopout();
      else toast.info('Only a chat next to a stream pops out.');
    }
  }

  const translating = $derived(settings.v.chat.translateChats.includes(login));
  const muted = $derived(settings.v.highlights.muted.includes(login));
  // 1.46 The shield shows only when this account moderates here.
  const modRole = $derived(mods.role(account, login));
  const modMode = $derived(mods.modMode(login));

  /** Adds or removes this chat from a list setting. */
  const toggled = (list: string[], on: boolean) => (on ? [...list, login] : list.filter((l) => l !== login));

  function record() {
    chatxIpc
      .record(room.view, !recording)
      .then((r) => {
        recording = r;
        toast.info(r ? 'Recording this chat' : 'Recording saved. Find it in Settings → Recordings.');
      })
      .catch((e) => toast.error(errorText(e)));
  }

  function more(e: MouseEvent) {
    const defs = settings.v.highlights.filters;
    const flip = (id: string) => onfilters?.(filters.includes(id) ? filters.filter((f) => f !== id) : [...filters, id]);
    const own = userMenus.current.channel;
    const ctx = { login, userId: null, display: name ?? login, msgId: null, msg: null, msgTime: null };
    // V101-30 Actions first, then switches; the search key is the one bound in Hotkeys.
    const items: MenuEntry[] = [
      { label: 'Search', icon: 'search', hint: bindingOf('chat.search'), run: () => (search = { query: '', all: false }) },
      { label: recording ? 'Stop recording' : 'Record chat', icon: 'record', run: record },
      // V102-20 Greyed out with its flag until the account can.
      ...(account
        ? [
            {
              label: 'Subscribe',
              icon: 'star' as const,
              lock: lockFor(accounts.can('wallet', account), 'helix') ?? lockFor(accounts.can('subs', account), 'gql'),
              run: () => (subscribing = true),
            },
          ]
        : []),
      ...(onoverlay ? [{ label: 'Overlay', icon: 'layers' as const, note: 'This chat over games, always on top', run: onoverlay }] : []),
      'separator',
      {
        label: 'Translate messages',
        icon: 'languages',
        checked: translating,
        run: () => void settings.patch('chat', { translateChats: toggled(settings.v.chat.translateChats, !translating) }),
      },
      {
        label: 'Mute sounds',
        icon: 'bellOff',
        checked: muted,
        run: () => void settings.patch('highlights', { muted: toggled(settings.v.highlights.muted, !muted) }),
      },
      ...(onfilters
        ? [
            {
              label: 'Filters',
              icon: 'filter' as const,
              items: defs.length
                ? defs.map((d) => ({ label: d.name, checked: filters.includes(d.id), run: () => flip(d.id) }))
                : [{ label: 'Make filters in Settings → Filters', disabled: true }],
            },
          ]
        : []),
      ...(own.length ? ['separator' as const, ...toItems(own, (cmd) => runEntry(room.view, cmd, ctx, onaction))] : []),
      // S9 Entries plugins put in a chat's menu.
      'separator',
      ...pluginItems(plugins.menus, 'chat', null, (p) =>
        pluginsIpc.runMenu(room.view, p.plugin, p.index, null).catch((err) => toast.error(errorText(err))),
      ),
      ...(tileMenu ? ['separator' as const, ...tileMenu()] : []),
    ];
    openMenu(e.currentTarget as HTMLElement, items, 'end');
  }

  const STATUS: Partial<Record<ConnStatus, string>> = {
    connecting: 'Connecting…',
    reconnecting: 'Reconnecting…',
    authFailed: 'Twitch didn’t accept this account. Sign in again in Settings → Accounts.',
    unavailable: 'This channel is suspended or doesn’t exist.',
    refused: 'Too many chats joined. Close a few, then open this one again.',
  };

  const modes = $derived.by(() => {
    const s = room.state;
    const list: { label: string; hint: string }[] = [];
    if (s.slow) list.push({ label: `Slow ${formatDuration(s.slow)}`, hint: 'One message per this long.' });
    if (s.followersOnly !== null)
      list.push({
        label: s.followersOnly ? `Followers ${formatDuration(s.followersOnly * 60)}` : 'Followers',
        hint: 'Only followers can chat.',
      });
    if (s.subsOnly) list.push({ label: 'Subs', hint: 'Only subscribers can chat.' });
    if (s.emoteOnly) list.push({ label: 'Emotes', hint: 'Emotes only.' });
    if (s.unique) list.push({ label: 'Unique', hint: 'Messages must be unique.' });
    return list;
  });

  function reply(id: string) {
    replyTo = room.find(id) ?? null;
  }

  function openChannelCard() {
    chatIpc.openUsercard(login, login).catch((e) => toast.error(errorText(e)));
  }
</script>

<!-- Remembers which chat was used last, for chat hotkeys. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<section
  class="chat"
  bind:this={el}
  onpointerdown={() => (lastUsed = el ?? null)}
  onfocusin={() => (lastUsed = el ?? null)}
>
  <!-- The header drags a tile in a tiled tab (U3); its buttons stay clickable. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={onheaderdown}>
    <button class="channel" onclick={openChannelCard} use:tooltip={{ label: 'Channel info', hint: 'Followers, roles and when they joined.' }}>{name ?? login}</button>
    <div class="modes">
      {#if recording}<span class="mode rec" use:tooltip={'Recording this chat'}>REC</span>{/if}
      {#if filters.length}<span class="mode" use:tooltip={{ label: 'Shown through filters', hint: 'Change them in ⋯ → Filters.' }}>Filtered</span>{/if}
      {#each modes as m (m.label)}
        <span class="mode" use:tooltip={{ label: m.label, hint: m.hint }}>{m.label}</span>
      {/each}
    </div>
    {#if modRole}
      <!-- V101-33 Mod mode: delete, timeout and ban in front of each line, per chat. -->
      <IconButton
        icon="sword"
        label={modMode ? 'Mod mode on' : 'Mod mode'}
        hint="Delete, timeout and ban buttons in front of each message."
        size={15}
        active={modMode}
        onclick={() => mods.setModMode(login, !modMode)}
      />
      {#if onmodview}
        <IconButton
          icon="shield"
          label={modviewOpen ? 'Close mod view' : 'Mod view'}
          hint="Mod tools for this chat."
          size={15}
          active={modviewOpen}
          onclick={onmodview}
        />
      {/if}
    {/if}
    <IconButton
      icon="users"
      label="Viewers"
      hint="Who’s in this chat."
      size={15}
      active={viewers}
      onclick={() => (viewers = !viewers)}
    />
    {@render actions?.()}
    <IconButton
      icon="more"
      label="More"
      hint="Search, translate, record, filters and more."
      size={15}
      onclick={more}
    />
  </header>
  {#if room.error}
    <p class="status bad">{room.error}</p>
  {:else if STATUS[room.status]}
    <p class="status" class:bad={['authFailed', 'unavailable', 'refused'].includes(room.status)}>
      {STATUS[room.status]}
    </p>
  {/if}
  <!-- V102-24 A short chat keeps its messages and message box: these scroll instead. -->
  <div class="banners">
    <EventsButton channel={login} pin={room.pinned} pinKnown={room.pinKnown} />
    {#if room.pinned}
      <PinnedBanner
        pin={room.pinned}
        channel={login}
        onunpin={modRole && !room.pinned.paid ? () => void chatIpc.send(room.view, '/unpin').catch((e) => toast.error(errorText(e))) : undefined}
      />
    {/if}
    <VotesBanner channel={login} view={() => room.view} signedIn={room.account !== null} />
  </div>
  <div class="body">
    <MessageList
      {room}
      {readKey}
      canReply={room.account !== null}
      onreply={reply}
      {onlink}
      onmention={(l) => input?.mention(l)}
      onprefill={(t) => input?.prefill(t)}
      oninsert={(t) => input?.insertText(t)}
      {onaction}
    />
    <!-- V101-35 Over the messages, so the list keeps its place underneath. -->
    {#if viewers}
      <div class="viewers"><ViewerList account={room.account} {login} onclose={() => (viewers = false)} /></div>
    {/if}
    {#if cosmeticsOpen && account}<CosmeticsPopup {account} onclose={closeCosmetics} />{/if}
  </div>
  <ResubBanner {account} {login} />
  <ChatInput bind:this={input} {room} bind:replyTo {onsignin} {onaction} />
  {#if subscribing && account}<SubDialog {account} {login} onclose={() => (subscribing = false)} />{/if}
  {#if search}
    <SearchPopup
      view={room.view}
      channel={login}
      query={search.query}
      all={search.all}
      onclose={() => (search = null)}
    />
  {/if}
</section>

<style>
  .chat {
    position: relative;
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  header {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 36px;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid var(--border);
  }
  .channel {
    padding: 2px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    font-weight: 700;
  }
  .channel:hover {
    background: var(--surface-hover);
  }
  .modes {
    flex: 1;
    display: flex;
    gap: 4px;
    overflow: hidden;
  }
  .mode {
    padding: 1px 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .mode.rec {
    color: var(--red);
    border-color: var(--red);
  }
  .banners {
    flex: none;
    max-height: 40%;
    overflow-y: auto;
  }
  .body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  /* The message box always shows. */
  .chat > :global(.input) {
    flex: none;
  }
  .viewers {
    position: absolute;
    inset: 0;
    z-index: 2;
    display: flex;
    background: var(--page);
  }
  .status {
    margin: 0;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
  }
  .status.bad {
    color: var(--yellow);
  }
</style>
