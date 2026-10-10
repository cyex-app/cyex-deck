<!-- Notifications (2.2, 1.26): the app's own alerts (followed channels going live, whispers,
     Twitch's bell) and Twitch's settings for the default account: its email, phone and
     on-Twitch switches, and what each followed channel sends. -->
<script lang="ts">
  import Avatar from '../../../components/Avatar.svelte';
  import Button from '../../../components/Button.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { TwitchChannelNotify } from '../../../ipc/bindings/TwitchChannelNotify';
  import type { TwitchNotifySwitch } from '../../../ipc/bindings/TwitchNotifySwitch';
  import { homeIpc } from '../../../ipc/browse';
  import { profileIpc } from '../../../ipc/notify';
  import { accounts } from '../../../lib/accounts.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { CHANNEL_STATES, group, PLATFORMS } from '../notify';

  const n = $derived(settings.v.notifications);
  const account = $derived(accounts.current);
  /** Twitch's own settings need a GQL sign-in. */
  const twitch = $derived(!!account && accounts.can('onsiteNotifications', account.userId));

  let switches = $state.raw<TwitchNotifySwitch[]>([]);
  let channels = $state.raw<TwitchChannelNotify[]>([]);
  let cursor = $state<string | null>(null);
  let loading = $state(false);
  let error = $state('');
  let query = $state('');

  const groups = $derived(group(switches));
  const q = $derived(query.trim().toLowerCase());
  const shown = $derived(channels.filter((c) => !q || c.login.includes(q) || c.name.toLowerCase().includes(q)));

  $effect(() => {
    const id = account?.userId;
    switches = [];
    channels = [];
    cursor = null;
    error = '';
    if (id) void load(id, false);
  });

  async function load(id: string, more: boolean) {
    loading = true;
    try {
      if (twitch) {
        const page = await profileIpc.notify(id, more ? cursor : null);
        if (!more) switches = page.switches;
        channels = more ? [...channels, ...page.channels] : page.channels;
        cursor = page.cursor;
      } else if (accounts.can('followedList', id)) {
        // No Twitch bell to read with only Helix: every follow alerts unless turned off here.
        const list = await homeIpc.follows(id);
        channels = list.map((c) => ({ id: c.id, login: c.login, name: c.name, avatar: c.avatar, state: 'ALWAYS' }));
      }
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  const patch = (p: object) => void settings.patch('notifications', p);

  /** Whether the app alerts for a channel: its own choice, else Twitch's bell. */
  const alerts = (c: TwitchChannelNotify) => n.live.channels[c.id] ?? c.state !== 'NEVER';

  function setAlert(c: TwitchChannelNotify, on: boolean) {
    // Matching the bell again drops the app's own choice.
    patch({ live: { channels: { [c.id]: on === (c.state !== 'NEVER') ? null : on } } });
  }

  async function setTwitchChannel(c: TwitchChannelNotify, state: string) {
    if (!account) return;
    const before = c.state;
    channels = channels.map((x) => (x.id === c.id ? { ...x, state } : x));
    try {
      await profileIpc.setChannel(account.userId, c.id, state);
    } catch (e) {
      channels = channels.map((x) => (x.id === c.id ? { ...x, state: before } : x));
      toast.error(String(e));
    }
  }

  async function setSwitch(category: string, platform: string, on: boolean) {
    if (!account) return;
    const flip = (v: boolean) =>
      (switches = switches.map((s) => (s.category === category && s.platform === platform ? { ...s, on: v } : s)));
    flip(on);
    try {
      await profileIpc.setSwitch(account.userId, category, platform, on);
    } catch (e) {
      flip(!on);
      toast.error(String(e));
    }
  }
</script>

<h2>Notifications</h2>

<h3 class="label">When channels go live</h3>
<Row label="Live alerts" hint="For channels the default account follows. Each one follows its Twitch bell unless you pick here.">
  <Toggle label="Live alerts" checked={n.live.enabled} onchange={(on) => patch({ live: { enabled: on } })} />
</Row>
<Row label="Pop-up" hint="A Windows notification. Click it to open the channel.">
  <Toggle label="Pop-up" checked={n.live.toast} disabled={!n.live.enabled} onchange={(on) => patch({ live: { toast: on } })} />
</Row>
<Row label="Sound">
  <Toggle label="Sound" checked={n.live.sound} disabled={!n.live.enabled} onchange={(on) => patch({ live: { sound: on } })} />
</Row>
<Row label="Quiet at start" hint="No alerts for channels that were already live when the app opened.">
  <Toggle label="Quiet at start" checked={n.live.skipAtStart} disabled={!n.live.enabled} onchange={(on) => patch({ live: { skipAtStart: on } })} />
</Row>

<h3 class="label">Whispers</h3>
<Row label="Pop-up" hint="While the app isn't in front.">
  <Toggle label="Whisper pop-up" checked={n.whispers.toast} onchange={(on) => patch({ whispers: { toast: on } })} />
</Row>
<Row label="Sound">
  <Toggle label="Whisper sound" checked={n.whispers.sound} onchange={(on) => patch({ whispers: { sound: on } })} />
</Row>
<Row label="Flash the taskbar">
  <Toggle label="Flash the taskbar" checked={n.whispers.flash} onchange={(on) => patch({ whispers: { flash: on } })} />
</Row>
<Row label="Show in chat" hint="Whispers also show as lines in the chats on screen.">
  <Toggle label="Show whispers in chat" checked={n.whispers.inline} onchange={(on) => patch({ whispers: { inline: on } })} />
</Row>

<h3 class="label">Twitch's bell</h3>
<Row label="Pop-ups" hint="When Twitch marks a notification as a pop-up, show it as a Windows notification too.">
  <Toggle label="Bell pop-ups" checked={n.bell.toast} onchange={(on) => patch({ bell: { toast: on } })} />
</Row>

{#if account}
  <h3 class="label">Channels</h3>
  <p class="note">
    {#if twitch}
      <b>Alert</b> is this app. <b>On Twitch</b> is what Twitch itself sends {account.displayName} for the channel.
    {:else}
      Sign in with GQL to see and change Twitch's own choice per channel.
    {/if}
  </p>
  {#if channels.length > 8}
    <input class="field filter" placeholder="Find a channel" bind:value={query} spellcheck="false" />
  {/if}
  {#if error}<p class="note err">{error}</p>{/if}
  <div class="channels">
    {#each shown as c (c.id)}
      <div class="channel">
        <Avatar src={c.avatar} name={c.name} size={24} />
        <span class="name">{c.name}</span>
        {#if twitch}
          <select class="field" aria-label="On Twitch" value={c.state} onchange={(e) => void setTwitchChannel(c, e.currentTarget.value)}>
            {#each CHANNEL_STATES as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
          </select>
        {/if}
        <Toggle label="Alert" checked={alerts(c)} disabled={!n.live.enabled} onchange={(on) => setAlert(c, on)} />
      </div>
    {:else}
      {#if !loading && !error}<p class="note">No followed channels.</p>{/if}
    {/each}
  </div>
  {#if cursor}
    <Button small disabled={loading} onclick={() => account && void load(account.userId, true)}>Show more</Button>
  {/if}

  {#if twitch && groups.length}
    <h3 class="label">What Twitch sends</h3>
    <p class="note">Twitch's own settings for {account.displayName}, saved on Twitch.</p>
    <div class="grid">
      <span></span>
      {#each PLATFORMS as p (p.id)}<span class="head">{p.label}</span>{/each}
      {#each groups as g (g.label)}
        <span class="group">{g.label}</span>
        {#each g.categories as c (c.id)}
          <span class="cat">{c.label}</span>
          {#each PLATFORMS as p (p.id)}
            <span class="cell">
              {#if c.on[p.id] !== undefined}
                <Toggle label="{c.label}: {p.label}" checked={!!c.on[p.id]} onchange={(on) => void setSwitch(c.id, p.id, on)} />
              {/if}
            </span>
          {/each}
        {/each}
      {/each}
    </div>
  {/if}
{/if}

<style>
  .note {
    margin: 4px 0 10px;
    color: var(--muted);
    font-size: 0.82rem;
  }
  .err {
    color: var(--red);
  }
  .filter {
    width: 100%;
    margin-bottom: 8px;
  }
  .channels {
    display: flex;
    flex-direction: column;
    gap: 2px;
    margin-bottom: 8px;
  }
  .channel {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 6px;
    border-radius: var(--radius);
  }
  .channel:hover {
    background: var(--row-hover);
  }
  .channel .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .channel select {
    width: 120px;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr repeat(3, 76px);
    align-items: center;
    row-gap: 4px;
  }
  .head {
    color: var(--dim);
    font-size: 0.76rem;
    text-align: center;
  }
  .group {
    grid-column: 1 / -1;
    margin-top: 12px;
    color: var(--muted);
    font-size: 0.8rem;
    font-weight: 600;
  }
  .cat {
    padding-left: 4px;
  }
  .cell {
    display: flex;
    justify-content: center;
  }
</style>
