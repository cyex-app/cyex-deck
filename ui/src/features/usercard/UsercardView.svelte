<!-- Usercard (1.36), laid out like the 7TV extension's (V101-05): who someone is on top, your
     buttons (S1), block and hide, the mod bar where you moderate (1.57), then one scroll
     window that fills the rest of the fixed-height card: their messages here and in the
     logs, and for mods their timeouts, bans, warnings and mod comments (M3). Your own
     nickname and note for them (C3) open from the pencil. Route: `usercard/<channel>/<login>`. -->
<script lang="ts">
  import { onDestroy, untrack } from 'svelte';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { ModHistory } from '../../ipc/bindings/ModHistory';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { catalogs } from '../chat/catalog.svelte';
  import ActionList from './ActionList.svelte';
  import Actions from './Actions.svelte';
  import Comments from './Comments.svelte';
  import Facts from './Facts.svelte';
  import Header from './Header.svelte';
  import ModBar from './ModBar.svelte';
  import Timeline from './Timeline.svelte';

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
  // 1.57 The account (if any) that moderates this channel, for the mod bar and tabs.
  let modAccount = $state<string | null>(null);
  let history = $state<ModHistory | null>(null);
  let tab = $state<Tab>('messages');
  let editing = $state(false);
  let logsUrl = $state('');
  let settle: ReturnType<typeof setTimeout> | undefined;

  // svelte-ignore state_referenced_locally
  if (channel && login !== channel) {
    void modIpc.moderating(channel).then((a) => (modAccount = a)).catch(() => {});
  }

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
      error = String(e);
      profile = null;
    }
  }

  // Their messages from the open chat, refreshed while the card is open.
  $effect(() => {
    const id = profile?.id;
    if (!id) return;
    const refresh = () =>
      chatIpc
        .userMessages(channel, id)
        .then((m) => (messages = m))
        .catch(() => {});
    void refresh();
    const t = setInterval(refresh, REFRESH_MS);
    return () => clearInterval(t);
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

  function saveNote(field: 'nickname' | 'note', value: string) {
    if (!profile) return;
    const current = note ?? { nickname: '', note: '' };
    if (current[field] === value.trim()) return;
    void settings.patch('chat', { users: { [profile.id]: { ...current, [field]: value.trim() } } });
  }
</script>

<div class="card">
  {#if profile === undefined}
    <p class="muted">Loading…</p>
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
          <ModBar {profile} {channel} account={modAccount} ban={history ? history.ban : undefined} onchange={changed} />
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
