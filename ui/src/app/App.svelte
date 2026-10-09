<!-- Main window: title bar with the fixed tabs, then the active view. Channel tabs
     join the bar in phase 4. -->
<script lang="ts">
  import IconButton from '../components/IconButton.svelte';
  import Toasts from '../components/Toasts.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import AccountMenu from '../features/accounts/AccountMenu.svelte';
  import ConnectionCounter from '../features/accounts/ConnectionCounter.svelte';
  import DiscoverView from '../features/discover/DiscoverView.svelte';
  import HomeView from '../features/home/HomeView.svelte';
  import SettingsView from '../features/settings/SettingsView.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { currentWindow } from '../lib/window.svelte';
  import { popOut } from './panels';
  import { useRootEffects } from './root.svelte';

  type View = 'home' | 'discover' | 'settings';

  let view = $state<View>('home');
  let settingsSection = $state('general');

  useRootEffects();
  useHotkeys({
    'nav.home': () => (view = 'home'),
    'nav.discover': () => (view = 'discover'),
    'settings.open': () => (view = 'settings'),
    'window.pin': () => currentWindow.togglePin(),
  });

  function manageAccounts() {
    settingsSection = 'accounts';
    view = 'settings';
  }

  async function popOutSettings() {
    if (await popOut('settings')) view = 'home';
  }
</script>

<WindowChrome>
  {#snippet start()}
    <nav class="fixed-tabs">
      <IconButton
        icon="house"
        label="Home"
        hint="Channels you follow, live ones first."
        active={view === 'home'}
        onclick={() => (view = 'home')}
      />
      <IconButton
        icon="compass"
        label="Discover"
        hint="Find new streams and categories."
        active={view === 'discover'}
        onclick={() => (view = 'discover')}
      />
    </nav>
  {/snippet}
  {#snippet end()}
    <ConnectionCounter />
    <AccountMenu onmanage={manageAccounts} />
    <IconButton icon="settings" label="Settings" active={view === 'settings'} onclick={() => (view = 'settings')} />
  {/snippet}
</WindowChrome>

<main>
  {#if view === 'home'}
    <HomeView />
  {:else if view === 'discover'}
    <DiscoverView />
  {:else}
    <SettingsView onpopout={popOutSettings} bind:section={settingsSection} />
  {/if}
</main>

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
  main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
</style>
