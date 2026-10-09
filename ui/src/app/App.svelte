<!-- Main window: title bar with the fixed views and the channel tabs (02), the tab bar on
     the side set in Settings (U5), then the view or tab on screen. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import IconButton from '../components/IconButton.svelte';
  import MenuHost from '../components/MenuHost.svelte';
  import Toasts from '../components/Toasts.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import AccountMenu from '../features/accounts/AccountMenu.svelte';
  import ConnectionCounter from '../features/accounts/ConnectionCounter.svelte';
  import DiscoverView from '../features/discover/DiscoverView.svelte';
  import HomeView from '../features/home/HomeView.svelte';
  import SettingsView from '../features/settings/SettingsView.svelte';
  import { openAdd } from '../features/tabs/dialog.svelte';
  import PoppedOut from '../features/tabs/PoppedOut.svelte';
  import Switcher from '../features/tabs/Switcher.svelte';
  import TabBar from '../features/tabs/TabBar.svelte';
  import TabOverlays from '../features/tabs/TabOverlays.svelte';
  import TabView from '../features/tabs/TabView.svelte';
  import { tabs } from '../features/tabs/tabs.svelte';
  import { chatIpc } from '../ipc/chat';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { settings } from '../lib/settings.svelte';
  import { mentionChime } from '../lib/sound';
  import { currentWindow } from '../lib/window.svelte';
  import { popOut } from './panels';
  import { useRootEffects } from './root.svelte';

  type Fixed = 'home' | 'discover' | 'settings';

  /** A fixed view, or `null` while a tab shows. */
  let fixed = $state<Fixed | null>(tabs.active ? null : 'home');
  /** The tab picked here, shown before Rust confirms it as the active one. */
  let picked = $state<string | null>(tabs.active);
  let settingsSection = $state('general');
  let switcher = $state(false);

  const side = $derived(settings.v.appearance.tabBarSide);
  const current = $derived(fixed ? null : tabs.tab(picked) ? picked : tabs.active);
  const tab = $derived(tabs.tab(current));

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
    'settings.open': () => show('settings'),
    'window.pin': () => currentWindow.togglePin(),
    'tabs.switcher': () => (switcher = true),
    'tabs.new': () => openAdd(),
    'tabs.close': () => current && tabs.close([current]),
    'tabs.reopen': () => void tabs.reopen().then((id) => id && open(id)),
    'tabs.next': () => step(1),
    'tabs.prev': () => step(-1),
  });

  $effect(() => {
    const off = chatIpc.onMention(() => mentionChime());
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

  function manageAccounts() {
    settingsSection = 'accounts';
    show('settings');
  }

  async function popOutSettings() {
    if (await popOut('settings')) show('home');
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
    </nav>
    {#if side === 'top'}<TabBar {current} onopen={open} />{/if}
  {/snippet}
  {#snippet end()}
    <IconButton icon="search" label="Go to" hint="Jump to a tab or open a channel (Ctrl+K)." onclick={() => (switcher = true)} />
    <ConnectionCounter />
    <AccountMenu onmanage={manageAccounts} />
    <IconButton icon="settings" label="Settings" active={fixed === 'settings'} onclick={() => show('settings')} />
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
      <DiscoverView />
    {:else if fixed === 'settings'}
      <SettingsView onpopout={popOutSettings} bind:section={settingsSection} />
    {:else}
      <HomeView />
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
  .fixed-tabs {
    display: flex;
    gap: 2px;
    padding-right: 8px;
    margin-right: 4px;
    border-right: 1px solid var(--border);
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
    background: var(--bg);
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
