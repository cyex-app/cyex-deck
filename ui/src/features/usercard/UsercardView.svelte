<!-- Usercard (1.36, shell): who someone is, their last messages here, and your own
     nickname and note for them (C3). Route: `usercard/<channel>/<login>`. Sub status,
     follow age, name history and logs come in phase 8. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { avatars } from '../emotes/avatars.svelte';
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Profile } from '../../ipc/bindings/Profile';
  import { chatIpc } from '../../ipc/chat';
  import { systemIpc } from '../../ipc/system';
  import { formatDate, formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { catalogs } from '../chat/catalog.svelte';
  import Line from '../chat/Line.svelte';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const [, channel = '', login = ''] = route.split('/');
  const REFRESH_MS = 3000;

  let profile = $state<Profile | null | undefined>(undefined);
  let messages = $state.raw<ChatMessage[]>([]);
  let error = $state('');

  const note = $derived(profile ? settings.v.chat.users[profile.id] : undefined);
  const localized = $derived(profile ? profile.displayName.toLowerCase() !== profile.login : false);
  const isChannel = $derived(login === channel);

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

  // Last messages from the open chat, refreshed while the card is open.
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

  function saveNote(field: 'nickname' | 'note', value: string) {
    if (!profile) return;
    const current = note ?? { nickname: '', note: '' };
    if (current[field] === value.trim()) return;
    void settings.patch('chat', { users: { [profile.id]: { ...current, [field]: value.trim() } } });
  }

  async function copyId() {
    if (!profile) return;
    await navigator.clipboard.writeText(profile.id);
    toast.info('User ID copied.');
  }
</script>

<div class="card">
  {#if profile === undefined}
    <p class="muted">Loading…</p>
  {:else if profile === null}
    <p class="muted">{error || `No user called ${login}.`}</p>
  {:else}
    <header>
      <Avatar src={avatars.of(profile.login) ?? profile.avatar ?? ''} name={profile.displayName} size={56} />
      <div class="who">
        <div class="name">
          <strong>{note?.nickname || profile.displayName}</strong>
          {#if localized || note?.nickname}<span class="muted">{profile.login}</span>{/if}
        </div>
        <button class="id" onclick={copyId} use:tooltip={'Copy user ID'}>ID {profile.id}</button>
        <div class="roles">
          {#if profile.staff}<span class="role staff">Staff</span>{/if}
          {#if profile.partner}<span class="role partner">Partner</span>{/if}
          {#if profile.affiliate}<span class="role affiliate">Affiliate</span>{/if}
        </div>
      </div>
    </header>

    <dl>
      <dt>Created</dt>
      <dd>{profile.createdAt ? formatDate(new Date(profile.createdAt)) : '—'}</dd>
      <dt>Followers</dt>
      <dd>{profile.followers != null ? formatNumber(profile.followers) : '—'}</dd>
    </dl>

    {#if !isChannel}
      <button class="link" onclick={() => systemIpc.openLink(`https://www.twitch.tv/${profile!.login}`)}>
        <Icon name="popout" size={13} /> Open profile on Twitch
      </button>
    {/if}

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
      rows="2"
      placeholder="Private, kept on this PC"
      value={note?.note ?? ''}
      onchange={(e) => saveNote('note', e.currentTarget.value)}
    ></textarea>

    <h3 class="label">Last messages in {channel}</h3>
    <div class="messages">
      {#each messages as m (m.id)}
        <Line {m} {channel} />
      {:else}
        <p class="muted">Nothing here since you opened the chat.</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .card {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 12px 14px;
    user-select: text;
  }
  header {
    display: flex;
    gap: 12px;
    align-items: center;
  }
  .who {
    min-width: 0;
  }
  .name {
    display: flex;
    gap: 6px;
    align-items: baseline;
    font-size: 1.05rem;
  }
  .id {
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
    font: 0.75rem var(--font-mono);
  }
  .id:hover {
    color: var(--text);
  }
  .roles {
    display: flex;
    gap: 4px;
    margin-top: 4px;
  }
  .role {
    padding: 0 6px;
    border-radius: var(--radius);
    font-size: 0.7rem;
    font-weight: 700;
  }
  .staff {
    background: color-mix(in srgb, var(--red) 20%, transparent);
    color: var(--red);
  }
  .partner {
    background: color-mix(in srgb, var(--twitch) 20%, transparent);
    color: var(--twitch);
  }
  .affiliate {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  dl {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 2px 12px;
    margin: 12px 0;
    font-size: 0.85rem;
  }
  dt {
    color: var(--dim);
  }
  dd {
    margin: 0;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 10px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--accent-text);
    font-size: 0.85rem;
  }
  .link:hover {
    text-decoration: underline;
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
    resize: vertical;
  }
  .messages {
    margin: 0 -14px;
  }
  .muted {
    color: var(--dim);
  }
  p.muted {
    padding: 0 14px;
    font-size: 0.85rem;
  }
</style>
