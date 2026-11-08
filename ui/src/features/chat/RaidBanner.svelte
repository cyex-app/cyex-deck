<!-- V110-U43 A raid counting down from the channel this chat shows, in the pinned message's
     slot and look: who it goes to (avatar and name open their usercard), how many viewers
     go along (live from Twitch, about once a second) and the time left. It goes when the
     raid goes or is called off; Rust adds "Raided X with N viewers" to the chat. With
     Settings → Chat → "Join raid button" (off), signed-in chats get "Join raid": it joins
     through Twitch and opens the channel, stream and chat, in its own tab window, which can
     be closed or docked back into the tabs. Never joins by itself. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import { hideTip, tooltip } from '../../components/tooltip.svelte';
  import type { SmRaid } from '../../ipc/bindings/SmRaid';
  import { chatIpc } from '../../ipc/chat';
  import { raidIpc } from '../../ipc/raid';
  import { popOut } from '../../app/panels';
  import { nowSecond } from '../../lib/clock.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { tabRoute, tabs, tiles } from '../tabs/tabs.svelte';
  import { joinShows, raidLeft, viewersText } from './raidBanner';
  import { raids } from './raids.svelte';

  interface Props {
    /** The chat's channel login, for the usercard. */
    channel: string;
    channelId: string | null;
    /** The chat's account (user id); signed out: no Join. */
    account?: string | null;
  }

  let { channel, channelId, account = null }: Props = $props();

  $effect(() => {
    const id = channelId;
    if (!id) return;
    return raids.follow(id);
  });

  const raid = $derived(raids.of(channelId));
  const left = $derived(raid ? raidLeft(raid.goesAt, nowSecond()) : '');

  /** The raid joined from here, so Join doesn't show twice. */
  let joined = $state<string | null>(null);
  let busy = $state(false);
  const canJoin = $derived(joinShows(settings.v.chat.raidJoin, raid?.id ?? null, account, joined));

  function usercard(login: string) {
    hideTip();
    chatIpc.openUsercard(channel, login).catch((err) => toast.error(errorText(err)));
  }

  function cardKey(e: KeyboardEvent, login: string) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    usercard(login);
  }

  /** The raided channel's own tab (only it in there), or a new one, then in its own window. */
  async function openPopped(r: SmRaid, who: string | null) {
    if (!r.targetId) return;
    const own = tabs.list.find((t) => {
      const list = tiles(t);
      return list.length === 1 && list[0].channel.id === r.targetId;
    });
    let id = own?.id ?? null;
    if (!id) {
      // Adding makes the new tab active; the main window goes back to what it showed.
      const before = tabs.active;
      id = await tabs.apply({
        op: 'add',
        channel: { id: r.targetId, login: r.targetLogin, name: r.targetName },
        show: { player: true, chat: true },
        account: who,
        alias: null,
        into: null,
      });
      if (!id) return;
      tabs.activate(before);
    }
    await popOut(tabRoute(id), r.targetName);
  }

  async function join() {
    const r = raid;
    const who = account;
    if (!r || !who || busy) return;
    busy = true;
    try {
      await raidIpc.join(who, r.id);
      joined = r.id;
      await openPopped(r, who);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }
</script>

{#if raid}
  <div class="raid" role="status" aria-label="Raiding {raid.targetName}, {viewersText(raid.viewers, formatNumber)}, {left === 'now' ? 'going now' : `${left} left`}">
    <Icon name="users" size={14} />
    <span
      class="who"
      role="link"
      tabindex="0"
      aria-label="{raid.targetName}, open usercard"
      use:tooltip={{ label: raid.targetName, hint: 'Opens their usercard.' }}
      onclick={() => usercard(raid.targetLogin)}
      onkeydown={(e) => cardKey(e, raid.targetLogin)}
    >
      <Avatar src={raid.targetAvatar} name={raid.targetName} size={20} />
      <span class="text"><span class="dim">Raiding</span> <strong>{raid.targetName}</strong></span>
    </span>
    <span class="meta">{viewersText(raid.viewers, formatNumber)}</span>
    <span class="left">{left}</span>
    {#if canJoin}
      <Button small variant="primary" disabled={busy} onclick={join}>Join raid</Button>
    {/if}
  </div>
{/if}

<style>
  .raid {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 6px 8px 2px;
    padding: 5px 8px 5px 10px;
    min-height: 32px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: calc(var(--chat-size) * 0.9);
    line-height: 1.3;
  }
  .raid > :global(svg) {
    flex: none;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    min-width: 0;
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .who:hover strong {
    text-decoration: underline;
  }
  .who:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
  }
  .text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text strong {
    color: var(--text);
    font-weight: 700;
  }
  .dim {
    color: var(--dim);
  }
  .meta,
  .left {
    flex: none;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .left {
    color: var(--dim);
    font-size: var(--text-hint);
    min-width: 3ch;
    text-align: right;
  }
</style>
