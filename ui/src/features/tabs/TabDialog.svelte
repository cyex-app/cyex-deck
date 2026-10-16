<!-- Add or edit a tab (1.14, 1.16, 1.17, 1.18, 1.22): the channel, stream and/or chat,
     the account it joins on, and an optional tab name. Also adds a channel into an
     existing tab (U3), or a feed tab (F4), and can put the channel in the miner (1.34). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import Checkbox from '../../components/Checkbox.svelte';
  import Dialog from '../../components/Dialog.svelte';
  import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
  import type { TileShow } from '../../ipc/bindings/TileShow';
  import { minerIpc } from '../../ipc/miner';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import AccountPicker from './AccountPicker.svelte';
  import ChannelPicker from './ChannelPicker.svelte';
  import { closeTabDialog, type TabDialogMode } from './dialog.svelte';
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
  let feed = $state<string | null>(null);
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

  const title = editing ? 'Edit tab' : into ? `Add a channel to ${tab ? label(tab) : 'this tab'}` : 'Add a tab';
  const named = !into;

  /** Another tab that already shows this channel. */
  const existing = $derived(
    picked ? tabs.list.find((t) => tiles(t).some((x) => x.channel.id === picked!.id && x.id !== tile?.id)) : undefined,
  );

  const ready = $derived(feed !== null ? !!feed.trim() && !busy : !!picked && (show.player || show.chat) && !busy);

  async function submit() {
    if (feed !== null) return void addFeed(feed);
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

  async function addFeed(name: string) {
    if (!name.trim()) return;
    busy = true;
    const id = await tabs.apply({ op: 'addFeed', name });
    busy = false;
    if (id) onopen(id);
    closeTabDialog();
  }

  function goTo(id: string) {
    onopen(id);
    closeTabDialog();
  }
</script>

<Dialog title={feed !== null ? 'Add a feed tab' : title} width={440} top onclose={closeTabDialog} onsubmit={submit}>
  {#if feed !== null}
    <div class="form">
      <section>
        <span class="label">Feed name</span>
        <input
          class="field"
          maxlength="60"
          placeholder="Mentions"
          bind:value={feed}
        />
        <p class="note">
          Highlight rules with “Also show in” and this name send their messages here, from every open chat.
        </p>
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
        <div class="row">
          <Checkbox
            label="Stream"
            checked={show.player}
            disabled={show.player && !show.chat}
            onchange={(v) => (show = { ...show, player: v })}
          />
          <Checkbox
            label="Chat"
            checked={show.chat}
            disabled={show.chat && !show.player}
            onchange={(v) => (show = { ...show, chat: v })}
          />
        </div>
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
        <Button variant="link" small onclick={() => (feed = '')}>Make a feed tab instead</Button>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={closeTabDialog}>Cancel</Button>
    <Button variant="primary" disabled={!ready} onclick={submit}>
      {editing ? 'Save' : into ? 'Add channel' : feed !== null ? 'Add feed' : 'Add tab'}
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
  .row {
    display: flex;
    gap: 18px;
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
