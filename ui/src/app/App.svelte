<!-- Main window: title bar with the fixed views and the channel tabs (02), the tab bar on
     the side set in Settings (U5), then the view or tab on screen. Below 900 px wide, top
     tabs get a full-width row under the title bar (F03). A tab left while its
     stream plays keeps playing out of sight (V102-15). A view popped out of here comes back
     where it was when its window closes (V102-16). -->
<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { untrack } from 'svelte';

  import IconButton from '../components/IconButton.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import Toasts from '../components/Toasts.svelte';
  import UnlockDialog from '../features/unlock/UnlockDialog.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import AccountMenu from '../features/accounts/AccountMenu.svelte';
  import BellButton from '../features/bell/BellButton.svelte';
  import { follow } from '../features/bell/follow';
  import ConnectionCounter from '../features/accounts/ConnectionCounter.svelte';
  import DiscoverView from '../features/discover/DiscoverView.svelte';
  import HomeView from '../features/home/HomeView.svelte';
  import { miner } from '../features/miner/miner.svelte';
  import MinerView from '../features/miner/MinerView.svelte';
  import ModViewPanel from '../features/modview/ModViewPanel.svelte';
  import { modviewPlace, modviewRoute } from '../features/modview/where.svelte';
  import SettingsView from '../features/settings/SettingsView.svelte';
  import StreamManagerView from '../features/streammanager/StreamManagerView.svelte';
  import SubsView from '../features/subs/SubsView.svelte';
  import CosmeticsView from '../features/cosmetics/CosmeticsView.svelte';
  import { whispers } from '../features/whispers/whispers.svelte';
  import { isMentionsFeed, MENTIONS_FEED } from '../features/chat/mentionsTab';
  import { mentionsTab } from '../features/chat/mentionsTab.svelte';
  import { roomRoute } from '../features/chat/roomRoute';
  import WhispersView from '../features/whispers/WhispersView.svelte';
  import { openAdd } from '../features/tabs/dialog.svelte';
  import PoppedOut from '../features/tabs/PoppedOut.svelte';
  import Switcher from '../features/tabs/Switcher.svelte';
  import TabBar from '../features/tabs/TabBar.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import TabView from '../features/tabs/TabView.svelte';
  import UpdateBadge from '../features/update/UpdateBadge.svelte';
  import { openTarget, playerHotkeys } from '../features/player/players.svelte';
  import { openChannel } from '../features/browse/open.svelte';
  import { barPlace } from '../features/tabs/barLayout';
  import { stepTab } from '../features/tabs/cycle';
  import { nextHeld, playingNow } from '../features/tabs/hold';
  import { tabs, tiles } from '../features/tabs/tabs.svelte';
  import type { Tab } from '../ipc/bindings/Tab';
  import type { LinkTarget } from '../ipc/bindings/LinkTarget';
  import { accountsIpc } from '../ipc/accounts';
  import { notifyEvents } from '../ipc/notify';
  import { soundIpc } from '../ipc/sound';
  import { pluginsIpc } from '../ipc/plugins';
  import { tabsIpc } from '../ipc/tabs';
  import { windowsIpc } from '../ipc/windows';
  import { accounts } from '../lib/accounts.svelte';
  import { errorText } from '../lib/errors';
  import { gate } from '../lib/gate.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { followFullscreen } from '../lib/layers';
  import { settings } from '../lib/settings.svelte';
  import { streamer } from '../lib/streamer.svelte';
  import { decodeSound } from '../lib/sound';
  import { toast } from '../lib/toast.svelte';
  import { currentWindow } from '../lib/window.svelte';
  import { popOut } from './panels';
  import { useRootEffects } from './root.svelte';

  type Fixed = 'home' | 'discover' | 'miner' | 'streamManager' | 'whispers' | 'subs' | 'cosmetics' | 'settings' | 'modview';
  /** Somewhere the main window can show. */
  type Place = { tab: string } | { view: Fixed };

  /** A fixed view, or `null` while a tab shows. */
  let fixed = $state<Fixed | null>(tabs.active ? null : 'home');
  /** The tab picked here, shown before Rust confirms it as the active one. */
  let picked = $state<string | null>(tabs.active);
  let settingsSection = $state('general');
  let switcher = $state(false);

  const side = $derived(settings.v.appearance.tabBarSide);
  let width = $state(innerWidth);
  /** F03 Where the tab bar goes now. */
  const place = $derived(barPlace(side, width));
  const current = $derived(fixed ? null : tabs.tab(picked) ? picked : tabs.active);
  const tab = $derived(tabs.tab(current));
  /** The tab drawn here now, not in its own window. */
  const shownTab = $derived(tab && !tabs.popped.has(tab.id) ? tab.id : null);

  // V102-15 Which tab plays out of sight, worked out before the old tab leaves the page. A
  // tab picked while it's in its own window counts too: its live stream plays there.
  let shownBefore: string | null = null;
  $effect.pre(() => {
    const to = current;
    const now = shownTab;
    untrack(() => {
      const held = tabs.held && tabs.tab(tabs.held) && !tabs.popped.has(tabs.held) ? tabs.held : null;
      tabs.held = nextHeld({ held, from: shownBefore, to, playing, liveViewer });
      shownBefore = now;
    });
  });

  // Closed or popped out: nothing left to keep playing here.
  $effect(() => {
    const h = tabs.held;
    if (h && (!tabs.tab(h) || tabs.popped.has(h))) tabs.held = null;
  });

  /** The tab on screen, and the one playing out of sight before it so it never moves. */
  const mounted = $derived.by(() => {
    const list: Tab[] = [];
    const held = tabs.held && tabs.held !== shownTab ? tabs.tab(tabs.held) : undefined;
    if (held && !tabs.popped.has(held.id)) list.push(held);
    const shown = tabs.tab(shownTab);
    if (shown) list.push(shown);
    return list;
  });

  /** A stream or video plays in the tab now, muted or not. */
  function playing(id: string): boolean {
    return playingNow(document.querySelector(`[data-tab-view="${CSS.escape(id)}"]`)?.querySelectorAll('video') ?? []);
  }

  /** The tab shows a stream and that channel is live. */
  function liveViewer(id: string): boolean {
    const t = tabs.tab(id);
    return Boolean(t && tiles(t).some((x) => x.show.player && tabs.live[x.channel.id]?.stream));
  }
  /** 08 The miner works once an account has a GQL sign-in; until then it's greyed out (V102-20). */
  const canMine = $derived(accounts.any('miner'));
  /** 10 The stream manager shows once an account can run its own channel (D26). */
  const canManage = $derived(accounts.any('streamManager'));
  /** 1.26 Whispers show once the default account can whisper. */
  const canWhisper = $derived(accounts.can('whispers'));

  // Follow the active tab when an edit moves it, e.g. closing the shown tab.
  $effect(() => {
    const active = tabs.active;
    if (active) untrack(() => (picked = active));
  });

  // The last tab closed: back to home.
  $effect(() => {
    if (!fixed && !tab) fixed = 'home';
  });

  useRootEffects();
  useHotkeys({
    'nav.home': () => show('home'),
    'nav.discover': () => show('discover'),
    'nav.miner': () => (canMine ? show('miner') : gate.ask('gql')),
    'nav.streamManager': () => canManage && show('streamManager'),
    'nav.whispers': () => canWhisper && show('whispers'),
    'settings.open': () => show('settings'),
    'window.pin': () => currentWindow.togglePin(),
    'tabs.switcher': () => (switcher = true),
    'tabs.new': () => openAdd(),
    'tabs.close': () => current && tabs.close([current]),
    'tabs.reopen': () => void tabs.reopen().then((id) => id && open(id)),
    'tabs.next': () => step(1),
    'tabs.prev': () => step(-1),
    ...playerHotkeys,
  });

  $effect(() => {
    void miner.start();
  });

  // V1051-01 Rust plays the app's sounds; this window decodes a sound file the first time.
  $effect(() => {
    const off = soundIpc.onDecode((d) => void decodeSound(d));
    return () => void off.then((f) => f());
  });

  // V104-01 "Open the Mentions tab" in Settings: the feed tab named Mentions, added if
  // there's none.
  $effect(() => {
    if (!mentionsTab.asks) return;
    untrack(async () => {
      const have = tabs.list.find((t) => tiles(t).some((x) => isMentionsFeed(x.feed)));
      const id = have?.id ?? (await tabs.apply({ op: 'addFeed', name: MENTIONS_FEED }));
      if (id) open(id);
    });
  });

  $effect(() => {
    void whispers.start();
  });

  // V102-20 A greyed-out tool's flag, in any window: Settings → Accounts on that sign-in.
  $effect(() => {
    const off = accountsIpc.onShowSignIn((kind) => {
      manageAccounts();
      gate.shown(kind);
    });
    return () => void off.then((f) => f());
  });

  // 1.26 Someone asked for a conversation: a name's menu, an inline line, `/w`.
  $effect(() => {
    if (whispers.asks) untrack(() => show('whispers'));
  });

  // 1.46 The shield on a chat here: that channel's mod view.
  $effect(() => {
    if (modviewPlace.asks) untrack(() => show('modview'));
  });

  // V102-16 Views popped out of here, by route: what they were and where this window went.
  const docked = new Map<string, { view: Fixed; at: Place }>();
  $effect(() => {
    const off = windowsIpc.onPanels((routes) => {
      for (const [route, d] of docked) {
        if (routes.includes(route)) continue;
        docked.delete(route);
        if (d.view === 'modview') modviewPlace.login = route.slice(route.indexOf('/') + 1);
        // Back where it was, unless you went somewhere else meanwhile.
        if (isHere(d.at)) show(d.view);
      }
    });
    return () => void off.then((f) => f());
  });

  // 2.2, 1.26 Toasts: a click opens the channel, the conversation or the link.
  $effect(() => {
    const open = notifyEvents.onOpen((o) => {
      if (o.kind === 'channel') void openLogin(o.login);
      else if (o.kind === 'whisper') whispers.show(o.login);
      else if (o.kind === 'room') void popOut(roomRoute(o.login, null), `${o.login} chat`);
      else follow(o.url, (l) => void openLogin(l));
    });
    return () => {
      void open.then((f) => f());
    };
  });

  // 1.58 A clip or VOD linked in a popped-out chat opens with the stream here.
  $effect(() => {
    const off = listen<{ tile: string; target: LinkTarget }>('tile:link', (e) => {
      const tab = tabs.tabOf(e.payload.tile);
      const tile = tab && tiles(tab).find((t) => t.id === e.payload.tile);
      if (!tab || !tile) return;
      openTarget(tile, e.payload.target);
      if (!tabs.popped.has(tab.id)) open(tab.id);
    });
    return () => void off.then((f) => f());
  });

  // S13 A plugin asks for a tab or a channel. A popped-out tab stays in its own window.
  $effect(() => {
    const off = pluginsIpc.onOpen((o) => {
      if (o.kind === 'channel') void openChannel({ show: open, account: () => accounts.current?.userId ?? null }, o.channel);
      else if (tabs.tab(o.id) && !tabs.popped.has(o.id)) open(o.id);
    });
    return () => void off.then((f) => f());
  });

  /** Where the main window was before the place it shows now. */
  let before: Place | null = null;

  function here(): Place | null {
    if (fixed) return { view: fixed };
    return current ? { tab: current } : null;
  }

  function isHere(p: Place): boolean {
    return 'tab' in p ? !fixed && current === p.tab : fixed === p.view;
  }

  function moved() {
    const now = here();
    if (now) before = now;
  }

  function show(view: Fixed) {
    if (fixed !== view) moved();
    fixed = view;
    tabs.activate(null);
  }

  function open(id: string) {
    if (fixed || current !== id) moved();
    picked = id;
    fixed = null;
    tabs.activate(id);
  }

  /** V1051-02 Cancel on the 7TV consent, Escape in Settings: back to where you were. */
  function leaveView(view: Fixed) {
    const back = before && !('view' in before && before.view === view) ? before : { view: 'home' as const };
    if ('tab' in back && tabs.tab(back.tab) && !tabs.popped.has(back.tab)) open(back.tab);
    else show('view' in back ? back.view : 'home');
  }

  /** V102-16 Pops a view out of here and goes back to where you were before it; when its
   * window closes the view comes back, if you're still there. */
  async function popOutView(view: Fixed, route: string, title?: string) {
    if (!(await popOut(route, title))) return;
    const back: Place = before && !('view' in before && before.view === view) ? before : { view: 'home' };
    if ('tab' in back && tabs.tab(back.tab) && !tabs.popped.has(back.tab)) open(back.tab);
    else show('view' in back ? back.view : 'home');
    docked.set(route, { view, at: here() ?? { view: 'home' } });
  }

  /** Next or previous visible tab, wrapping around. */
  function step(by: number) {
    const next = stepTab(tabs.shownIds(current), current, by, true);
    if (next) open(next);
  }

  /** Opens a channel by login, as a tab of its own. */
  async function openLogin(login: string) {
    try {
      const c = (await tabsIpc.lookup(login)).find((x) => x.login === login.toLowerCase());
      if (!c) return toast.error(`No channel called ${login}.`);
      await openChannel({ show: open, account: () => accounts.current?.userId ?? null }, c);
    } catch (e) {
      toast.error(errorText(e, 'open that channel'));
    }
  }

  function manageAccounts() {
    settingsSection = 'accounts';
    show('settings');
  }

  function minerSettings() {
    settingsSection = 'miner';
    show('settings');
  }

  /** U4 Why streamer mode is on (or was turned off) and what a click does. */
  const streamerHint = $derived(
    !streamer.active
      ? 'Turned off by hand. Click to turn it back on.'
      : streamer.state.forced
        ? 'Turned on by hand. Click to turn it off.'
        : streamer.state.appRunning
          ? 'Your streaming app is running, so links, pictures and mod actions stay off your stream. Click to turn it off for now.'
          : 'Links, pictures and mod actions stay off your stream. Click to turn it off for now.',
  );

  const popOutManager = () => void popOutView('streamManager', 'streammanager');
  const popOutWhispers = (login: string | null) =>
    void popOutView('whispers', login ? `whispers/${login}` : 'whispers');
  function popOutModview() {
    const login = modviewPlace.login;
    if (login) void popOutView('modview', modviewRoute(login), `Mod view · ${login}`);
  }
</script>

<WindowChrome>
  {#snippet start()}
    <nav class="fixed-tabs">
      <IconButton
        icon="house"
        label="Home"
        hint="Channels you follow, live ones first."
        active={fixed === 'home'}
        onclick={() => show('home')}
      />
      <IconButton
        icon="compass"
        label="Discover"
        hint="Find new streams and categories."
        active={fixed === 'discover'}
        onclick={() => show('discover')}
      />
      <IconButton
        icon="pickaxe"
        label="Miner"
        hint={miner.mining.length ? `Mining ${miner.mining.join(' and ')}.` : 'Collects channel points while you do other things.'}
        active={fixed === 'miner'}
        badge={miner.mining.length > 0}
        lock={canMine ? null : 'gql'}
        onclick={() => show('miner')}
      />
      {#if canManage}
        <IconButton
          icon="radio"
          label="Stream manager"
          hint="Run your own stream: title, ads, polls, raids and more."
          active={fixed === 'streamManager'}
          onclick={() => show('streamManager')}
        />
      {/if}
    </nav>
    {#if place === 'chrome'}<TabBar chrome {current} onopen={open} />{/if}
  {/snippet}
  {#snippet end()}
    {#if streamer.active || streamer.state.forced}
      <IconButton
        icon="eyeOff"
        label={streamer.active ? 'Streamer mode is on' : 'Streamer mode is off for now'}
        hint={streamerHint}
        active={streamer.active}
        onclick={() => streamer.toggle()}
      />
    {/if}
    <IconButton icon="search" label="Go to" hint="Jump to a tab or open a channel (Ctrl+K)." onclick={() => (switcher = true)} />
    <BellButton onchannel={(l) => void openLogin(l)} />
    {#if canWhisper}
      <div class="with-count">
        <IconButton
          icon="whisper"
          label="Whispers"
          hint={whispers.state.unread ? `${whispers.state.unread} unread` : 'Your whispers (Ctrl+5).'}
          active={fixed === 'whispers'}
          onclick={() => show('whispers')}
        />
        {#if whispers.state.unread}<span class="count">{whispers.state.unread > 99 ? '99+' : whispers.state.unread}</span>{/if}
      </div>
    {/if}
    <ConnectionCounter />
    <AccountMenu onmanage={manageAccounts} onsubs={() => show('subs')} oncosmetics={() => show('cosmetics')} active={fixed === 'cosmetics'} />
    <UpdateBadge />
    <IconButton icon="settings" label="Settings" hint="Everything you can change (Ctrl+,)." active={fixed === 'settings'} onclick={() => show('settings')} />
  {/snippet}
</WindowChrome>

<svelte:window bind:innerWidth={width} />

{#if place === 'row'}
  <div class="tab-row">
    <TabBar {current} onopen={open} />
  </div>
{/if}

<div class="body {side}">
  {#if side !== 'top'}
    <div class="side-bar">
      <TabBar vertical={side !== 'bottom'} {current} onopen={open} />
    </div>
  {/if}
  <main>
    {#each mounted as t (t.id)}
      <div class="slot" class:away={t.id !== shownTab}>
        <TabView tab={t} background={t.id !== shownTab} onsignin={manageAccounts} />
      </div>
    {/each}
    {#if tab}
      {#if tabs.popped.has(tab.id)}
        <PoppedOut {tab} />
      {/if}
    {:else if fixed === 'modview' && modviewPlace.login}
      {#key modviewPlace.login}
        <ModViewPanel route={modviewRoute(modviewPlace.login)} onpopout={popOutModview} />
      {/key}
    {:else if fixed === 'discover'}
      <DiscoverView onopen={open} />
    {:else if fixed === 'miner' && canMine}
      <MinerView onsettings={minerSettings} />
    {:else if fixed === 'streamManager' && canManage}
      <StreamManagerView onpopout={popOutManager} onsignin={manageAccounts} />
    {:else if fixed === 'whispers' && canWhisper}
      <WhispersView onpopout={popOutWhispers} onsignin={manageAccounts} />
    {:else if fixed === 'subs'}
      <SubsView onchannel={(c) => void openChannel({ show: open, account: () => accounts.current?.userId ?? null }, c)} onsignin={manageAccounts} />
    {:else if fixed === 'cosmetics'}
      <CosmeticsView onclose={() => leaveView('cosmetics')} />
    {:else if fixed === 'settings'}
      <SettingsView bind:section={settingsSection} onclose={() => leaveView('settings')} />
    {:else}
      <HomeView onopen={open} ondiscover={() => show('discover')} onsignin={manageAccounts} />
    {/if}
  </main>
</div>

{#if switcher}
  <Switcher
    onopen={open}
    onview={show}
    onsettings={(page) => {
      settingsSection = page;
      show('settings');
    }}
    onclose={() => (switcher = false)} />
{/if}
<TabOverlays onopen={open} />
<!-- V101-26 Kept on screen in fullscreen. -->
<div class="layers" use:followFullscreen>
  <MenuHost />
  <TooltipHost />
  <UnlockDialog />
  <Toasts />
</div>

<style>
  .layers {
    display: contents;
  }
  /* Spans every tab row; the buttons stay on the first. */
  .fixed-tabs {
    display: flex;
    align-self: stretch;
    align-items: flex-start;
    gap: 2px;
    padding: 3.5px 8px 0 0;
    margin-right: 4px;
    border-right: 1px solid var(--border);
  }
  /* V101-19 Up to the window's top edge. */
  .fixed-tabs :global(.icon-btn::after) {
    content: '';
    position: absolute;
    inset: -3.5px 0;
  }
  .with-count {
    position: relative;
  }
  .count {
    position: absolute;
    top: 1px;
    right: 0;
    min-width: 15px;
    height: 15px;
    padding: 0 4px;
    border-radius: 8px;
    background: var(--accent);
    color: var(--accent-ink);
    font-size: 10px;
    font-weight: 700;
    line-height: 15px;
    text-align: center;
    pointer-events: none;
    box-shadow: 0 0 0 2px var(--bg);
  }
  .body {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .body.right {
    flex-direction: row-reverse;
  }
  .body.bottom {
    flex-direction: column-reverse;
  }
  .side-bar {
    display: flex;
    min-height: 0;
    background: var(--page);
  }
  .left .side-bar {
    border-right: 1px solid var(--border);
  }
  .right .side-bar {
    border-left: 1px solid var(--border);
  }
  .bottom .side-bar {
    padding: 3px 6px;
    border-top: 1px solid var(--border);
  }
  /* F03 Top tabs in a narrow window: a full-width row under the title bar. */
  .tab-row {
    display: flex;
    flex-direction: column;
    flex: none;
    padding: 0 6px;
    background: var(--page);
    border-bottom: 1px solid var(--border);
  }
  main {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  .slot {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
  /* V102-15 Playing out of sight. */
  .away {
    display: none;
  }
</style>
