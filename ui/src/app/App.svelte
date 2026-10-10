<!-- Main window: title bar with the fixed views and the channel tabs (02), the tab bar on
     the side set in Settings (U5), then the view or tab on screen. -->
<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { untrack } from 'svelte';

  import IconButton from '../components/IconButton.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import Toasts from '../components/Toasts.svelte';
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
  import SettingsView from '../features/settings/SettingsView.svelte';
  import StreamManagerView from '../features/streammanager/StreamManagerView.svelte';
  import SubsView from '../features/subs/SubsView.svelte';
  import { whispers } from '../features/whispers/whispers.svelte';
  import WhispersView from '../features/whispers/WhispersView.svelte';
  import { openAdd } from '../features/tabs/dialog.svelte';
  import PoppedOut from '../features/tabs/PoppedOut.svelte';
  import Switcher from '../features/tabs/Switcher.svelte';
  import TabBar from '../features/tabs/TabBar.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import TabView from '../features/tabs/TabView.svelte';
  import { openTarget, playerHotkeys } from '../features/player/players.svelte';
  import { openChannel } from '../features/browse/open.svelte';
  import { tabs, tiles } from '../features/tabs/tabs.svelte';
  import type { LinkTarget } from '../ipc/bindings/LinkTarget';
  import { chatIpc } from '../ipc/chat';
  import { notifyEvents } from '../ipc/notify';
  import { tabsIpc } from '../ipc/tabs';
  import { accounts } from '../lib/accounts.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { settings } from '../lib/settings.svelte';
  import { streamer } from '../lib/streamer.svelte';
  import { mentionChime } from '../lib/sound';
  import { toast } from '../lib/toast.svelte';
  import { currentWindow } from '../lib/window.svelte';
  import { popOut } from './panels';
  import { useRootEffects } from './root.svelte';

  type Fixed = 'home' | 'discover' | 'miner' | 'streamManager' | 'whispers' | 'subs' | 'settings';

  /** A fixed view, or `null` while a tab shows. */
  let fixed = $state<Fixed | null>(tabs.active ? null : 'home');
  /** The tab picked here, shown before Rust confirms it as the active one. */
  let picked = $state<string | null>(tabs.active);
  let settingsSection = $state('general');
  let switcher = $state(false);

  const side = $derived(settings.v.appearance.tabBarSide);
  const current = $derived(fixed ? null : tabs.tab(picked) ? picked : tabs.active);
  const tab = $derived(tabs.tab(current));
  /** 08 The miner shows once an account has a GQL sign-in. */
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
    'nav.miner': () => canMine && show('miner'),
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

  $effect(() => {
    const off = chatIpc.onMention(() => mentionChime());
    return () => void off.then((f) => f());
  });

  $effect(() => {
    void whispers.start();
  });

  // 1.26 Someone asked for a conversation: a name's menu, an inline line, `/w`.
  $effect(() => {
    if (whispers.asks) untrack(() => show('whispers'));
  });

  // 2.2, 1.26 Toasts: a click opens the channel, the conversation or the link; the sound
  // plays here.
  $effect(() => {
    const open = notifyEvents.onOpen((o) => {
      if (o.kind === 'channel') void openLogin(o.login);
      else if (o.kind === 'whisper') whispers.show(o.login);
      else follow(o.url, (l) => void openLogin(l));
    });
    const sound = notifyEvents.onSound(() => mentionChime());
    return () => {
      void open.then((f) => f());
      void sound.then((f) => f());
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

  function show(view: Fixed) {
    fixed = view;
    tabs.activate(null);
  }

  function open(id: string) {
    picked = id;
    fixed = null;
    tabs.activate(id);
  }

  /** Next or previous visible tab, wrapping around. */
  function step(by: number) {
    const list = tabs.list.filter((t) => !tabs.group(t.group)?.collapsed || t.id === current);
    if (!list.length) return;
    const at = list.findIndex((t) => t.id === current);
    const next = at < 0 ? (by > 0 ? 0 : list.length - 1) : (at + by + list.length) % list.length;
    open(list[next].id);
  }

  /** Opens a channel by login, as a tab of its own. */
  async function openLogin(login: string) {
    try {
      const c = (await tabsIpc.lookup(login)).find((x) => x.login === login.toLowerCase());
      if (!c) return toast.error(`No channel called ${login}.`);
      await openChannel({ show: open, account: () => accounts.current?.userId ?? null }, c);
    } catch (e) {
      toast.error(String(e));
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

  async function popOutSettings() {
    if (await popOut('settings')) show('home');
  }

  async function popOutManager() {
    if (await popOut('streammanager')) show('home');
  }

  async function popOutWhispers() {
    if (await popOut('whispers')) show('home');
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
      {#if canMine}
        <IconButton
          icon="pickaxe"
          label="Miner"
          hint={miner.mining.length ? `Mining ${miner.mining.join(' and ')}.` : 'Collects channel points while you do other things.'}
          active={fixed === 'miner'}
          badge={miner.mining.length > 0}
          onclick={() => show('miner')}
        />
      {/if}
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
    {#if side === 'top'}<TabBar chrome {current} onopen={open} />{/if}
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
    <AccountMenu onmanage={manageAccounts} onsubs={() => show('subs')} />
    <IconButton icon="settings" label="Settings" hint="Everything you can change (Ctrl+,)." active={fixed === 'settings'} onclick={() => show('settings')} />
  {/snippet}
</WindowChrome>

<div class="body {side}">
  {#if side !== 'top'}
    <div class="side-bar">
      <TabBar vertical={side !== 'bottom'} {current} onopen={open} />
    </div>
  {/if}
  <main>
    {#if tab}
      {#if tabs.popped.has(tab.id)}
        <PoppedOut {tab} />
      {:else}
        {#key tab.id}
          <TabView {tab} onsignin={manageAccounts} />
        {/key}
      {/if}
    {:else if fixed === 'discover'}
      <DiscoverView onopen={open} />
    {:else if fixed === 'miner' && canMine}
      <MinerView onsettings={minerSettings} onsignin={manageAccounts} />
    {:else if fixed === 'streamManager' && canManage}
      <StreamManagerView onpopout={popOutManager} onsignin={manageAccounts} />
    {:else if fixed === 'whispers' && canWhisper}
      <WhispersView onpopout={popOutWhispers} onsignin={manageAccounts} />
    {:else if fixed === 'subs'}
      <SubsView onchannel={(c) => void openChannel({ show: open, account: () => accounts.current?.userId ?? null }, c)} onsignin={manageAccounts} />
    {:else if fixed === 'settings'}
      <SettingsView onpopout={popOutSettings} bind:section={settingsSection} />
    {:else}
      <HomeView onopen={open} ondiscover={() => show('discover')} onsignin={manageAccounts} />
    {/if}
  </main>
</div>

{#if switcher}
  <Switcher onopen={open} onview={show} onclose={() => (switcher = false)} />
{/if}
<TabOverlays onopen={open} />
<MenuHost />
<TooltipHost />
<Toasts />

<style>
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
  main {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }
</style>
