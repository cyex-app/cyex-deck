<!-- Notifications (2.2, 1.26): the app's own alerts (followed channels going live, whispers,
     Twitch's bell) and Twitch's settings for the default account: what each followed channel
     sends, and its email, phone and on-Twitch switches as closed groups under Twitch's own
     group switches (V101-06). -->
<script lang="ts">
  import Avatar from '../../../components/Avatar.svelte';
  import Button from '../../../components/Button.svelte';
  import GateFlag from '../../../components/GateFlag.svelte';
  import Icon from '../../../components/Icon.svelte';
  import Row from '../../../components/Row.svelte';
  import Toggle from '../../../components/Toggle.svelte';
  import type { TwitchChannelNotify } from '../../../ipc/bindings/TwitchChannelNotify';
  import type { TwitchNotifySwitch } from '../../../ipc/bindings/TwitchNotifySwitch';
  import { homeIpc } from '../../../ipc/browse';
  import { profileIpc } from '../../../ipc/notify';
  import { accounts } from '../../../lib/accounts.svelte';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import { CHANNEL_STATES, countOn, group, PLATFORMS, sends, smart, type Category, type Group } from '../notify';
  import { errorText } from '../../../lib/errors';
  import Select from '../../../components/Select.svelte';

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
  let showChannels = $state(false);
  let opened = $state<string[]>([]);

  const sw = $derived(group(switches));
  const smartOn = $derived(smart(switches));
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
      error = errorText(e);
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
      toast.error(errorText(e));
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
      toast.error(errorText(e));
    }
  }

  const has = (c: Category | null | undefined, p: string) => c?.on[p] !== undefined;
  /** A group of one is a single row: it switches the group and its one setting together. */
  const single = (g: Group) => g.categories.length === 1;
  const isOn = (g: Group, p: string) =>
    single(g) ? g.master?.on[p] !== false && !!g.categories[0].on[p] : !!g.master?.on[p];
  const groupHas = (g: Group, p: string) => (g.master ? has(g.master, p) : single(g) && has(g.categories[0], p));

  function setGroup(g: Group, p: string, on: boolean) {
    if (g.master && has(g.master, p) && g.master.on[p] !== on) void setSwitch(g.master.id, p, on);
    const only = g.categories[0];
    if (single(g) && has(only, p) && only.on[p] !== on) void setSwitch(only.id, p, on);
  }

  /** Some switches in a closed group are off where the group itself is on. */
  const mixed = (g: Group) =>
    PLATFORMS.some((p) => {
      const [on, all] = countOn(g, p.id);
      return sends(sw, g, p.id) && on < all;
    });

  function toggleOpen(label: string) {
    opened = opened.includes(label) ? opened.filter((l) => l !== label) : [...opened, label];
  }
</script>

<h2>Notifications</h2>

<h3 class="label">When channels go live</h3>
<Row setting="notifications.live.enabled" label="Live alerts" hint="For channels the default account follows. Each one follows its Twitch bell unless you pick below.">
  <Toggle label="Live alerts" checked={n.live.enabled} onchange={(on) => patch({ live: { enabled: on } })} />
</Row>
{#if n.live.enabled}
  <Row setting="notifications.live.toast" label="Pop-up" hint="A Windows notification. Click it to open the channel.">
    <Toggle label="Pop-up" checked={n.live.toast} onchange={(on) => patch({ live: { toast: on } })} />
  </Row>
  <Row setting="notifications.live.sound" label="Sound">
    <Toggle label="Sound" checked={n.live.sound} onchange={(on) => patch({ live: { sound: on } })} />
  </Row>
  <Row setting="notifications.live.skipAtStart" label="Quiet at start" hint="No alerts for channels that were already live when the app opened.">
    <Toggle label="Quiet at start" checked={n.live.skipAtStart} onchange={(on) => patch({ live: { skipAtStart: on } })} />
  </Row>
{/if}

<h3 class="label">Whispers</h3>
<Row setting="notifications.whispers.toast" label="Pop-up" hint="While the app isn’t in front.">
  <Toggle label="Whisper pop-up" checked={n.whispers.toast} onchange={(on) => patch({ whispers: { toast: on } })} />
</Row>
<Row setting="notifications.whispers.sound" label="Sound">
  <Toggle label="Whisper sound" checked={n.whispers.sound} onchange={(on) => patch({ whispers: { sound: on } })} />
</Row>
<Row setting="notifications.whispers.flash" label="Flash the taskbar">
  <Toggle label="Flash the taskbar" checked={n.whispers.flash} onchange={(on) => patch({ whispers: { flash: on } })} />
</Row>
<Row setting="notifications.whispers.inline" label="Show in chat" hint="Whispers also show as lines in the chats on screen.">
  <Toggle label="Show whispers in chat" checked={n.whispers.inline} onchange={(on) => patch({ whispers: { inline: on } })} />
</Row>

<h3 class="label">Twitch’s bell</h3>
<Row setting="notifications.bell.toast" label="Pop-ups" hint="When Twitch marks a notification as a pop-up, show it as a Windows notification too.">
  <Toggle label="Bell pop-ups" checked={n.bell.toast} onchange={(on) => patch({ bell: { toast: on } })} />
</Row>

{#if account}
  <h3 class="label" data-setting="notifications.live.channels">Per channel</h3>
  <button class="fold" aria-expanded={showChannels} onclick={() => (showChannels = !showChannels)}>
    <Icon name={showChannels ? 'chevronDown' : 'chevronRight'} size={14} />
    <span>{channels.length ? `${channels.length}${cursor ? '+' : ''} followed channel${channels.length === 1 && !cursor ? '' : 's'}` : 'Followed channels'}</span>
    <span class="dim">{twitch ? 'Alerts here and what Twitch sends' : 'Alerts here'}</span>
  </button>
  {#if showChannels}
    <p class="note">
      {#if twitch}
        <b>Alert</b> is this app. <b>On Twitch</b> is what Twitch itself sends {account.displayName} for the channel.
      {:else}
        Twitch’s own choice per channel needs a GQL sign-in. <GateFlag kind="gql" />
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
            <Select label="On Twitch" width="120px" value={c.state} options={CHANNEL_STATES} onchange={(v) => void setTwitchChannel(c, v)} />
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
  {/if}

  {#if account && !twitch}
    <!-- V102-20 Greyed out, not hidden. -->
    <h3 class="label" data-setting="notifications:twitch">What Twitch sends</h3>
    <p class="note gated">Twitch’s own settings need a GQL sign-in. <GateFlag kind="gql" /></p>
  {/if}

  {#if twitch && sw.groups.length}
    <h3 class="label" data-setting="notifications:twitch">What Twitch sends</h3>
    <p class="note">Twitch’s own settings for {account.displayName}, saved on Twitch. Open a group to pick single ones.</p>
    {#if smartOn !== null}
      <Row label="Smart delivery" hint="Live alerts reach you once: on Twitch while you’re there, else by phone or email.">
        <Toggle label="Smart delivery" checked={smartOn} onchange={(on) => void setSwitch('LIVE', 'smart', on)} />
      </Row>
    {/if}
    <div class="grid">
      <span></span>
      {#each PLATFORMS as p (p.id)}<span class="head">{p.label}</span>{/each}
      {#if sw.all}
        {@const all = sw.all}
        <span class="cat strong single">Everything</span>
        {#each PLATFORMS as p (p.id)}
          <span class="cell">
            {#if has(all, p.id)}
              <Toggle label="Everything: {p.label}" checked={!!all.on[p.id]} onchange={(on) => void setSwitch(all.id, p.id, on)} />
            {/if}
          </span>
        {/each}
      {/if}
      {#each sw.groups as g (g.label)}
        {@const open = opened.includes(g.label)}
        {#if single(g)}
          <span class="cat strong single">{g.label}</span>
        {:else}
          <button class="group" aria-expanded={open} onclick={() => toggleOpen(g.label)}>
            <Icon name={open ? 'chevronDown' : 'chevronRight'} size={14} />
            <span class="strong">{g.label}</span>
            {#if !open && mixed(g)}<span class="dim">Some off</span>{/if}
          </button>
        {/if}
        {#each PLATFORMS as p (p.id)}
          <span class="cell">
            {#if groupHas(g, p.id)}
              <Toggle
                label="{g.label}: {p.label}"
                checked={isOn(g, p.id)}
                disabled={sw.all?.on[p.id] === false}
                onchange={(on) => setGroup(g, p.id, on)}
              />
            {/if}
          </span>
        {/each}
        {#if open && !single(g)}
          {#each g.categories as c (c.id)}
            <span class="cat sub">{c.label}</span>
            {#each PLATFORMS as p (p.id)}
              <span class="cell">
                {#if has(c, p.id)}
                  <Toggle
                    label="{c.label}: {p.label}"
                    checked={!!c.on[p.id]}
                    disabled={!sends(sw, g, p.id)}
                    onchange={(on) => void setSwitch(c.id, p.id, on)}
                  />
                {/if}
              </span>
            {/each}
          {/each}
        {/if}
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
  .gated {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
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
  .fold,
  .group {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    padding: 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    font: inherit;
    text-align: left;
  }
  .fold {
    width: 100%;
    margin-bottom: 6px;
  }
  .fold:hover,
  .group:hover {
    background: var(--row-hover);
  }
  .fold :global(svg),
  .group :global(svg) {
    flex: none;
    color: var(--dim);
  }
  .dim {
    margin-left: auto;
    overflow: hidden;
    color: var(--dim);
    font-size: 0.76rem;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .grid {
    display: grid;
    grid-template-columns: minmax(0, 1fr) repeat(3, 76px);
    align-items: center;
    row-gap: 4px;
  }
  .head {
    color: var(--dim);
    font-size: 0.76rem;
    text-align: center;
  }
  .cat {
    min-width: 0;
    padding-left: 4px;
  }
  .strong {
    font-weight: 600;
  }
  /* Lines up with the names of groups that open. */
  .single {
    padding-left: 24px;
  }
  .sub {
    padding-left: 44px;
    color: var(--muted);
  }
  .cell {
    display: flex;
    justify-content: center;
  }
</style>
