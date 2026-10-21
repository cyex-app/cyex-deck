<!-- Usercard (1.36), laid out like the 7TV extension's (V101-05): who someone is on top, your
     buttons (S1), block and hide, the mod bar where you moderate (1.57), then one scroll
     window that fills the rest of the card: their messages here and in the
     logs, and for mods their timeouts, bans, warnings and mod comments (M3). Your own
     nickname and note for them (C3) open from the pencil. Route: `usercard/<channel>/<login>`. -->
<script lang="ts">
  import { onDestroy, untrack } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { ModHistory } from '../../ipc/bindings/ModHistory';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import { accounts } from '../../lib/accounts.svelte';
  import { everyShown } from '../../lib/awake';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { ChatRoom } from '../chat/room.svelte';
  import { catalogs } from '../chat/catalog.svelte';
  import { mods } from '../modview/moderation.svelte';
  import { standingOf } from '../modview/standing';
  import ActionList from './ActionList.svelte';
  import Actions from './Actions.svelte';
  import Comments from './Comments.svelte';
  import Facts from './Facts.svelte';
  import Header from './Header.svelte';
  import ModBar from './ModBar.svelte';
  import Timeline from './Timeline.svelte';
  import { changedLines, theirLines } from './liveLines';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const [, channel = '', login = ''] = route.split('/');
  const REFRESH_MS = 3000;
  /** Twitch's mod logs take a moment to show an action. */
  const LOGS_SETTLE_MS = 1500;

  type Tab = 'messages' | 'timeouts' | 'bans' | 'warnings' | 'comments';
  const KINDS: Record<string, string> = { timeouts: 'TIMEOUT', bans: 'BAN', warnings: 'WARN' };

  let profile = $state<Profile | null | undefined>(undefined);
  let messages = $state.raw<ChatMessage[]>([]);
  let error = $state('');
  // 1.57 The account (if any) that moderates this channel, for the mod bar and tabs; live
  // (V102-21).
  const modAccount = $derived(channel && login !== channel ? mods.moderating(channel) : null);
  let history = $state<ModHistory | null>(null);
  let tab = $state<Tab>('messages');
  let editing = $state(false);
  let logsUrl = $state('');
  let settle: ReturnType<typeof setTimeout> | undefined;

  const note = $derived(profile ? settings.v.chat.users[profile.id] : undefined);
  const isChannel = $derived(login === channel);
  const last = $derived(messages.at(-1));

  void catalogs.loadGlobal();
  void catalogs.loadChannel(channel);
  void load();

  async function load() {
    try {
      profile = await chatIpc.profile(login);
    } catch (e) {
      error = errorText(e);
      profile = null;
    }
  }

  // Their messages from the open chat. V107-R23 With the room shown somewhere, its line
  // events bring a new line at once (a follows view, so it isn't counted as a chat);
  // otherwise it is read every few seconds while the window shows.
  let room = $state.raw<ChatRoom>();
  const readMessages = (id: string) =>
    chatIpc
      .userMessages(channel, id)
      .then((m) => (messages = m))
      .catch(() => {});
  $effect(() => {
    const id = profile?.id;
    if (!id) return;
    void readMessages(id);
    let dead = false;
    let stop = () => {};
    const poll = () => {
      // V107-H1 Not while the window is hidden: once when it shows.
      if (!dead) stop = everyShown(() => void readMessages(id), REFRESH_MS);
    };
    chatIpc
      .shown()
      .then((s) => {
        if (dead || !s.chats[channel]) return poll();
        const r = new ChatRoom(channel);
        void r.open(accounts.current?.userId ?? null, [], true);
        room = r;
        stop = () => {
          room = undefined;
          r.close();
        };
      })
      .catch(poll);
    return () => {
      dead = true;
      stop();
    };
  });
  let seen: ChatMessage[] = [];
  $effect(() => {
    const id = profile?.id;
    const list = room?.messages;
    if (!id || !list) return;
    const theirs = theirLines(list, id);
    if (changedLines(seen, theirs)) void readMessages(id);
    seen = theirs;
  });

  // M3 Mod history over GQL; silent where the account can't read it.
  function readHistory() {
    if (!modAccount || !profile) return;
    modIpc
      .history(modAccount, channel, profile.id)
      .then((h) => (history = h))
      .catch(() => {});
  }
  $effect(() => {
    if (modAccount && profile?.id) untrack(readHistory);
    // Unmodded while the card is open: the mod tabs go.
    else if (!modAccount) history = null;
  });
  function changed() {
    clearTimeout(settle);
    settle = setTimeout(readHistory, LOGS_SETTLE_MS);
  }
  onDestroy(() => clearTimeout(settle));

  const tabs = $derived<{ key: Tab; label: string; count: string }[]>(
    history
      ? [
          { key: 'messages', label: 'Messages', count: history.messageCount >= 1000 ? '1.000+' : formatNumber(history.messageCount) },
          { key: 'timeouts', label: 'Timeouts', count: formatNumber(history.timeouts) },
          { key: 'bans', label: 'Bans', count: formatNumber(history.bans) },
          { key: 'warnings', label: 'Warnings', count: formatNumber(history.warnings) },
          { key: 'comments', label: 'Comments', count: formatNumber(history.comments.length) },
        ]
      : [],
  );

  /** V107-R35 The card moves its window from anywhere that isn't a control, text the user
   *  types or the scroll window of messages and actions. */
  const NO_DRAG = 'button, a, input, textarea, select, label, summary, [role="button"], [role="tab"], [contenteditable], .data > :not(.tabs), .no-drag';
  function drag(e: MouseEvent) {
    if (e.button !== 0 || e.detail > 1) return;
    const t = e.target as Element | null;
    if (!t || t.closest(NO_DRAG)) return;
    e.preventDefault();
    void getCurrentWindow().startDragging();
  }

  function saveNote(field: 'nickname' | 'note', value: string) {
    if (!profile) return;
    const current = note ?? { nickname: '', note: '' };
    if (current[field] === value.trim()) return;
    void settings.patch('chat', { users: { [profile.id]: { ...current, [field]: value.trim() } } });
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="card" onmousedown={drag}>
  {#if profile === undefined}
    <Loading />
  {:else if profile === null}
    <p class="muted">{error || `No user called ${login}.`}</p>
  {:else}
    <div class="top">
      <Header
        {profile}
        {channel}
        badges={last?.sender?.badges ?? []}
        color={last?.sender?.color ?? null}
        {isChannel}
        {logsUrl}
        bind:editing
      />
      <Facts {profile} {channel} />
      {#if !isChannel}
        <Actions {profile} {channel} />
        {#if modAccount}
          <ModBar
            {profile}
            {channel}
            account={modAccount}
            ban={history ? history.ban : undefined}
            onchange={changed}
            is={standingOf(last?.sender?.badges)}
          />
        {/if}
      {/if}
    </div>

    <div class="data">
      {#if editing}
        <div class="note">
          <label class="label" for="nick">Nickname</label>
          <input
            id="nick"
            class="field"
            maxlength="25"
            placeholder="Only you see this"
            value={note?.nickname ?? ''}
            onchange={(e) => saveNote('nickname', e.currentTarget.value)}
          />
          <label class="label" for="note">Note</label>
          <textarea
            id="note"
            class="field"
            maxlength="500"
            rows="4"
            placeholder="Private, kept on this PC"
            value={note?.note ?? ''}
            onchange={(e) => saveNote('note', e.currentTarget.value)}
          ></textarea>
        </div>
      {:else if tabs.length}
        <nav class="tabs">
          {#each tabs as t (t.key)}
            <button class:on={tab === t.key} onclick={() => (tab = t.key)}>
              <span class="count">{t.count}</span>{t.label}
            </button>
          {/each}
        </nav>
      {/if}
      {#if history && !editing && tab === 'comments'}
        <Comments comments={history.comments} {channel} account={modAccount ?? ''} targetId={profile.id} onchange={changed} />
      {:else if history && !editing && tab !== 'messages'}
        <ActionList
          actions={history.actions.filter((a) => a.kind === KINDS[tab])}
          empty={`No ${tab} here.`}
        />
      {/if}
      <!-- Kept while other tabs show, so the logs aren't read again. -->
      <div class="pane" class:gone={editing || tab !== 'messages'}>
        <Timeline
          {channel}
          userId={profile.id}
          login={profile.login}
          name={note?.nickname || profile.displayName}
          live={messages}
          bind:url={logsUrl}
          {modAccount}
          account={modAccount ?? accounts.current?.userId ?? null}
        />
      </div>
    </div>
  {/if}
</div>

<style>
  .card {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    user-select: text;
  }
  /* Takes what it needs; if a mod with many buttons outgrows the card, it scrolls. */
  .top {
    flex: 0 1 auto;
    min-height: 0;
    overflow-y: auto;
  }
  .data {
    display: flex;
    flex: 1 0 160px;
    flex-direction: column;
    min-height: 0;
    border-top: 1px solid var(--border);
    background: var(--page);
  }
  .tabs {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    flex: none;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .tabs button {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 5px 2px 4px;
    background: none;
    border: 0;
    border-bottom: 2px solid transparent;
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .tabs button:hover {
    color: var(--text);
  }
  .tabs .on {
    border-bottom-color: var(--accent);
    color: var(--text);
  }
  .count {
    color: var(--dim);
    font-size: 0.68rem;
    font-weight: 500;
    font-variant-numeric: tabular-nums;
  }
  .pane {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-height: 0;
  }
  .pane.gone {
    display: none;
  }
  .note {
    flex: 1;
    overflow-y: auto;
    padding: 0 14px 10px;
  }
  .label {
    display: block;
    margin: 10px 0 4px;
  }
  .field {
    width: 100%;
  }
  textarea.field {
    height: auto;
    padding: 6px 10px;
    resize: none;
  }
  .muted {
    padding: 12px 14px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
