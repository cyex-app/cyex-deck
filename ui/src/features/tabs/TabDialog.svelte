<!-- Add or edit a tab (1.14, 1.16, 1.17, 1.18, 1.22): the channel, stream and/or chat,
     the account it joins on, and an optional tab name. Also adds a channel into an
     existing tab (U3), or a feed tab (F4), and can put the channel in the miner (1.34). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { FeedKind } from '../../ipc/bindings/FeedKind';
  import type { TileShow } from '../../ipc/bindings/TileShow';
  import { minerIpc } from '../../ipc/miner';
  import { errorText } from '../../lib/errors';
  import { toast } from '../../lib/toast.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import Select from '../../components/Select.svelte';
  import { isMentionsFeed, MENTIONS_FEED } from '../chat/mentionsTab';
  import AccountPicker from './AccountPicker.svelte';
  import ChannelPicker from './ChannelPicker.svelte';
  import { closeTabDialog, type TabDialogMode } from './dialog.svelte';
  import ShowPicker from './ShowPicker.svelte';
  import { label, tabs, tiles } from './tabs.svelte';

  interface Props {
    mode: TabDialogMode;
    /** Shows a tab in the window, e.g. the one just added. */
    onopen: (id: string) => void;
  }

  let { mode, onopen }: Props = $props();

  // The dialog is recreated for each mode, so these read it once.
  // svelte-ignore state_referenced_locally
  const editing = mode.kind === 'edit' ? mode : null;
  // svelte-ignore state_referenced_locally
  const into = mode.kind === 'add' ? mode.into : undefined;
  // svelte-ignore state_referenced_locally
  const prefill = mode.kind === 'add' ? (mode.text ?? '') : '';
  const tab = editing ? tabs.tab(editing.tab) : into ? tabs.tab(into) : undefined;
  const tile = editing && tab ? tiles(tab).find((t) => t.id === editing.tile) : undefined;
  const fallbackAccount = accounts.current?.userId ?? null;

  let text = $state(tile?.channel.login ?? prefill);
  let picked = $state<ChannelInfo | null>(
    tile
      ? (tabs.live[tile.channel.id] ?? { ...tile.channel, avatar: null, offlineImage: null, stream: null })
      : null,
  );
  let show = $state<TileShow>({ ...(tile?.show ?? tabs.lastShow) });
  let account = $state<string | null>(tile ? tile.account : fallbackAccount);
  let anon = $state(tile ? tile.account === null : fallbackAccount === null);
  let alias = $state(editing ? (tab?.alias ?? '') : '');
  let busy = $state(false);
  /** F4 Making a feed tab instead of a channel tab. */
  let feedMode = $state(false);
  let feedKind = $state<'mentions' | 'live' | 'automod' | 'whispers' | 'custom'>('mentions');
  let feedName = $state('');
  const FEEDS = [
    { value: 'mentions', label: 'Mentions' },
    { value: 'live', label: 'Live notifications' },
    { value: 'automod', label: 'AutoMod' },
    { value: 'whispers', label: 'Whispers' },
    { value: 'custom', label: 'Custom feed' },
  ];
  const FEED_NOTES: Record<string, string> = {
    mentions: 'Your mentions and replies, from every open chat.',
    live: 'Who goes live or offline: your tabs, and the channels you follow while live alerts are on.',
    automod: 'Held messages from the chats you moderate. Allow or deny them here.',
    whispers: 'Whispers you get while cyex deck runs.',
    custom: 'Highlight rules with this name as their custom feed send messages here, from every open chat.',
  };
  /** 1.34 */
  const canMine = accounts.list.some((a) => a.features.includes('miner'));
  const wasMined = !!tile && settings.v.miner.channels.some((c) => c.id === tile.channel.id);
  let mine = $state(wasMined);
  /** Adding a channel the miner has already. */
  const mined = $derived(!editing && !!picked && settings.v.miner.channels.some((c) => c.id === picked!.id));

  // Mined channels need an account.
  $effect(() => {
    if (mine && anon) anon = false;
  });

  /** Renaming a feed tab: only its name. */
  const renamingFeed = !!editing && !!tile?.feed;
  const title = renamingFeed ? 'Rename feed' : editing ? 'Edit tab' : into ? `Add a channel to ${tab ? label(tab) : 'this tab'}` : 'Add a tab';
  const named = !into;

  /** Another tab that already shows this channel. */
  const existing = $derived(
    picked ? tabs.list.find((t) => tiles(t).some((x) => x.channel.id === picked!.id && x.id !== tile?.id)) : undefined,
  );

  /** The tab that already is this feed. */
  const haveFeed = $derived(
    feedMode
      ? tabs.list.find((t) =>
          tiles(t).some((x) => {
            if (feedKind === 'mentions') return x.feedKind === 'rules' && isMentionsFeed(x.feed);
            if (feedKind === 'custom') {
              return x.feedKind === 'rules' && !!feedName.trim() && x.feed?.trim().toLowerCase() === feedName.trim().toLowerCase();
            }
            return !!x.feed && x.feedKind === feedKind;
          }),
        )
      : undefined,
  );

  const ready = $derived(
    renamingFeed
      ? !busy
      : feedMode
        ? !haveFeed && !busy && (feedKind !== 'custom' || !!feedName.trim())
        : !!picked && (show.player || show.chat) && !busy,
  );

  async function submit() {
    if (renamingFeed && tab) return void renameFeed(tab.id, tab.alias);
    if (feedMode) return void addFeed();
    if (!ready || !picked) return;
    busy = true;
    const channel = { id: picked.id, login: picked.login, name: picked.name };
    const who = anon ? null : (account ?? fallbackAccount);
    const name = alias.trim() || null;
    if (editing && tile && tab) {
      const changed =
        channel.id !== tile.channel.id ||
        show.player !== tile.show.player ||
        show.chat !== tile.show.chat ||
        who !== tile.account;
      if (changed) await tabs.apply({ op: 'edit', tile: tile.id, channel, show, account: who });
      if (name !== tab.alias) await tabs.apply({ op: 'rename', tab: tab.id, alias: name });
      if (wasMined && !mine) {
        void settings.patch('miner', { channels: settings.v.miner.channels.filter((c) => c.id !== tile.channel.id) });
      }
    } else {
      const id = await tabs.apply({ op: 'add', channel, show, account: who, alias: name, into: into ?? null });
      if (id) onopen(id);
    }
    if (mine) void minerIpc.add(channel);
    busy = false;
    closeTabDialog();
  }

  async function renameFeed(id: string, was: string | null) {
    busy = true;
    const name = alias.trim() || null;
    try {
      if (name !== was) await tabs.apply({ op: 'rename', tab: id, alias: name });
      closeTabDialog();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  async function addFeed() {
    if (!ready) return;
    busy = true;
    try {
      const name = feedKind === 'mentions' ? MENTIONS_FEED : feedKind === 'custom' ? feedName.trim() : '';
      const kind: FeedKind = feedKind === 'mentions' || feedKind === 'custom' ? 'rules' : feedKind;
      const id = await tabs.apply({ op: 'addFeed', name, kind });
      if (id) onopen(id);
      closeTabDialog();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function goTo(id: string) {
    onopen(id);
    closeTabDialog();
  }
</script>

<Dialog title={feedMode ? 'Add a feed tab' : title} width={440} top onclose={closeTabDialog} onsubmit={submit}>
  {#if renamingFeed}
    <div class="form">
      <section>
        <span class="label">Tab name</span>
        <input class="field" maxlength="60" placeholder={tile?.feed ?? ''} bind:value={alias} />
      </section>
    </div>
  {:else if feedMode}
    <div class="form">
      <section>
        <Select label="Feed" width="100%" options={FEEDS} value={feedKind} onchange={(v) => (feedKind = v as typeof feedKind)} />
        {#if feedKind === 'custom'}
          <input class="field" maxlength="40" placeholder="Feed name" aria-label="Feed name" bind:value={feedName} />
        {/if}
        <p class="note">{FEED_NOTES[feedKind]}</p>
        {#if haveFeed}
          <p class="note">
            You have this feed already.
            <Button variant="link" small onclick={() => goTo(haveFeed.id)}>Go to it</Button>
          </p>
        {/if}
      </section>
    </div>
  {:else}
    <div class="form">
      <section>
        <span class="label">Channel</span>
        <ChannelPicker bind:text bind:picked onsubmit={submit} />
        {#if existing}
          <p class="note">
            Already open in {label(existing)}.
            <Button variant="link" small onclick={() => goTo(existing.id)}>Go there</Button>
          </p>
        {/if}
      </section>

      <section>
        <span class="label">Show</span>
        <ShowPicker bind:show />
      </section>

      <section>
        <span class="label">Account</span>
        <AccountPicker bind:account bind:anon noAnon={mine} />
      </section>

      {#if canMine}
        <section>
          {#if mined}
            <p class="note">In the miner already.</p>
          {:else}
            <Checkbox label="Add to the miner" checked={mine} onchange={(v) => (mine = v)} />
            {#if mine}<p class="note">Goes at the bottom of the miner’s list. Mined channels need an account.</p>{/if}
          {/if}
        </section>
      {/if}

      {#if named}
        <section>
          <span class="label">Tab name</span>
          <input
            class="field"
            maxlength="60"
            placeholder={picked?.name ?? 'Channel name'}
            bind:value={alias}
          />
        </section>
      {/if}
      {#if !editing && !into}
        <Button variant="link" small onclick={() => (feedMode = true)}>Make a feed tab instead</Button>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={closeTabDialog}>Cancel</Button>
    <Button variant="primary" disabled={!ready} onclick={submit}>
      {editing ? 'Save' : into ? 'Add channel' : feedMode ? 'Add feed' : 'Add tab'}
    </Button>
  {/snippet}
</Dialog>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .field {
    width: 100%;
  }
  .note {
    margin: 2px 0 0;
    color: var(--muted);
    font-size: 0.82rem;
  }
</style>
