<!-- Main window: title bar with the fixed tabs and open chats, then the active view.
     The chat list here is a stand-in until phase 4 brings real channel tabs. -->
<script lang="ts">
  import IconButton from '../components/IconButton.svelte';
  import Icon from '../components/Icon.svelte';
  import Popover from '../components/Popover.svelte';
  import Toasts from '../components/Toasts.svelte';
  import TooltipHost from '../components/TooltipHost.svelte';
  import WindowChrome from '../components/WindowChrome.svelte';
  import AccountMenu from '../features/accounts/AccountMenu.svelte';
  import ConnectionCounter from '../features/accounts/ConnectionCounter.svelte';
  import ChatView from '../features/chat/ChatView.svelte';
  import DiscoverView from '../features/discover/DiscoverView.svelte';
  import HomeView from '../features/home/HomeView.svelte';
  import SettingsView from '../features/settings/SettingsView.svelte';
  import { chatIpc } from '../ipc/chat';
  import { accounts } from '../lib/accounts.svelte';
  import { useHotkeys } from '../lib/hotkeys.svelte';
  import { mentionChime } from '../lib/sound';
  import { toast } from '../lib/toast.svelte';
  import { currentWindow } from '../lib/window.svelte';
  import { popOut } from './panels';
  import { useRootEffects } from './root.svelte';

  interface OpenChat {
    key: string;
    login: string;
    account: string | null;
  }

  let view = $state('home');
  let settingsSection = $state('general');
  let chats = $state<OpenChat[]>([]);
  let adding = $state(false);
  let newLogin = $state('');
  let addField = $state<HTMLInputElement>();

  $effect(() => {
    if (adding) addField?.focus();
  });

  useRootEffects();
  useHotkeys({
    'nav.home': () => (view = 'home'),
    'nav.discover': () => (view = 'discover'),
    'settings.open': () => (view = 'settings'),
    'window.pin': () => currentWindow.togglePin(),
  });

  $effect(() => {
    const off = chatIpc.onMention(() => mentionChime());
    return () => void off.then((f) => f());
  });

  function manageAccounts() {
    settingsSection = 'accounts';
    view = 'settings';
  }

  async function popOutSettings() {
    if (await popOut('settings')) view = 'home';
  }

  function openChat(e: SubmitEvent) {
    e.preventDefault();
    const login = newLogin.trim().replace(/^#/, '').toLowerCase();
    if (!/^\w{1,25}$/.test(login)) {
      toast.error('That isn’t a Twitch channel name.');
      return;
    }
    const account = accounts.current?.userId ?? null;
    const key = `${login}/${account ?? 'anon'}`;
    if (!chats.some((c) => c.key === key)) chats.push({ key, login, account });
    view = `chat:${key}`;
    newLogin = '';
    adding = false;
  }

  function closeChat(key: string) {
    chats = chats.filter((c) => c.key !== key);
    if (view === `chat:${key}`) view = 'home';
  }

  async function popOutChat(c: OpenChat) {
    if (await popOut(`chat/${c.key}`, c.login)) closeChat(c.key);
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
    <nav class="chats">
      {#each chats as c (c.key)}
        <div class="chat-tab" class:active={view === `chat:${c.key}`}>
          <button class="open" onclick={() => (view = `chat:${c.key}`)}>{c.login}</button>
          <button class="x" aria-label="Close {c.login}" onclick={() => closeChat(c.key)}><Icon name="x" size={12} /></button>
        </div>
      {/each}
      <div class="add">
        <IconButton icon="plus" label="Open a chat" size={15} onclick={() => (adding = !adding)} />
        <Popover bind:open={adding} align="start">
          <form class="add-form" onsubmit={openChat}>
            <input class="field" placeholder="Channel name" bind:value={newLogin} bind:this={addField} />
          </form>
        </Popover>
      </div>
    </nav>
  {/snippet}
  {#snippet end()}
    <ConnectionCounter />
    <AccountMenu onmanage={manageAccounts} />
    <IconButton icon="settings" label="Settings" active={view === 'settings'} onclick={() => (view = 'settings')} />
  {/snippet}
</WindowChrome>

<main>
  {#each chats as c (c.key)}
    <div class="chat-view" class:hidden={view !== `chat:${c.key}`}>
      <ChatView login={c.login} account={c.account} onpopout={() => popOutChat(c)} onsignin={manageAccounts} />
    </div>
  {/each}
  {#if view === 'home'}
    <HomeView />
  {:else if view === 'discover'}
    <DiscoverView />
  {:else if view === 'settings'}
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
  .chats {
    display: flex;
    align-items: center;
    gap: 2px;
    min-width: 0;
  }
  .chat-tab {
    display: flex;
    align-items: center;
    height: 28px;
    border-radius: var(--radius);
  }
  .chat-tab:hover {
    background: var(--surface-hover);
  }
  .chat-tab.active {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .chat-tab .open {
    padding: 0 4px 0 10px;
    background: none;
    border: 0;
    font-weight: 600;
  }
  .chat-tab .x {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    margin-right: 4px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .chat-tab .x:hover {
    color: var(--text);
  }
  .add {
    position: relative;
  }
  .add-form {
    padding: 4px;
  }
  .add-form .field {
    width: 100%;
  }
  main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .chat-view {
    flex: 1;
    min-width: 0;
    display: flex;
  }
  .hidden {
    display: none;
  }
</style>
