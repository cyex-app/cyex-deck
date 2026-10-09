<!-- A channel's chat (1.8, 1.9): header with room modes and status, pinned message,
     messages, message box. The header carries the tile's buttons and drag handle. -->
<script lang="ts">
  import { onDestroy, type Snippet } from 'svelte';

  import { tooltip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { ConnStatus } from '../../ipc/bindings/ConnStatus';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import { chatIpc } from '../../ipc/chat';
  import { formatDuration } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { catalogs } from './catalog.svelte';
  import ChatInput from './ChatInput.svelte';
  import MessageList from './MessageList.svelte';
  import PinnedBanner from './PinnedBanner.svelte';
  import { ChatRoom } from './room.svelte';

  interface Props {
    login: string;
    /** Display name for the header. */
    name?: string;
    /** User id of the reading account; `null` reads anonymously. */
    account: string | null;
    /** Keeps the last-read line (C5) across tab switches. */
    readKey?: string;
    /** Buttons at the end of the header. */
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
    onsignin?: () => void;
    /** Clips and VODs linked in chat open here (1.19, 1.20, P7). */
    onlink?: (target: LinkTarget) => void;
  }

  let { login, name, account, readKey, actions, onheaderdown, onsignin = () => {}, onlink }: Props = $props();

  // A chat view shows one channel for its whole life; a different one is a new view.
  // svelte-ignore state_referenced_locally
  const room = new ChatRoom(login);
  // svelte-ignore state_referenced_locally
  void room.open(account);
  void catalogs.loadGlobal();
  // svelte-ignore state_referenced_locally
  void catalogs.loadChannel(login);
  onDestroy(() => room.close());

  let replyTo = $state<ChatMessage | null>(null);

  const STATUS: Partial<Record<ConnStatus, string>> = {
    connecting: 'Connecting…',
    reconnecting: 'Reconnecting…',
    authFailed: 'Twitch didn’t accept this account. Sign in again in Settings → Accounts.',
    unavailable: 'This channel is suspended or doesn’t exist.',
    budgetFull: 'This account is in 100 chats, Twitch’s limit. Close one to join this.',
  };

  const modes = $derived.by(() => {
    const s = room.state;
    const list: { label: string; hint: string }[] = [];
    if (s.slow) list.push({ label: `Slow ${formatDuration(s.slow)}`, hint: 'One message per this long.' });
    if (s.followersOnly !== null)
      list.push({
        label: s.followersOnly ? `Followers ${formatDuration(s.followersOnly * 60)}` : 'Followers',
        hint: 'Only followers can chat.',
      });
    if (s.subsOnly) list.push({ label: 'Subs', hint: 'Only subscribers can chat.' });
    if (s.emoteOnly) list.push({ label: 'Emotes', hint: 'Emotes only.' });
    if (s.unique) list.push({ label: 'Unique', hint: 'Messages must be unique.' });
    return list;
  });

  function reply(id: string) {
    replyTo = room.find(id) ?? null;
  }

  function openChannelCard() {
    chatIpc.openUsercard(login, login).catch((e) => toast.error(String(e)));
  }
</script>

<section class="chat">
  <!-- The header drags a tile in a tiled tab (U3); its buttons stay clickable. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={onheaderdown}>
    <button class="channel" onclick={openChannelCard} use:tooltip={'Channel info'}>{name ?? login}</button>
    <div class="modes">
      {#each modes as m (m.label)}
        <span class="mode" use:tooltip={{ label: m.label, hint: m.hint }}>{m.label}</span>
      {/each}
    </div>
    {@render actions?.()}
  </header>
  {#if room.error}
    <p class="status bad">{room.error}</p>
  {:else if STATUS[room.status]}
    <p class="status" class:bad={['authFailed', 'unavailable', 'budgetFull'].includes(room.status)}>
      {STATUS[room.status]}
    </p>
  {/if}
  {#if room.pinned}<PinnedBanner pin={room.pinned} channel={login} />{/if}
  <MessageList {room} {readKey} canReply={room.account !== null} onreply={reply} {onlink} />
  <ChatInput {room} bind:replyTo {onsignin} />
</section>

<style>
  .chat {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 4px;
    height: 36px;
    padding: 0 6px 0 8px;
    border-bottom: 1px solid var(--border);
  }
  .channel {
    padding: 2px 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    font-weight: 700;
  }
  .channel:hover {
    background: var(--surface-hover);
  }
  .modes {
    flex: 1;
    display: flex;
    gap: 4px;
    overflow: hidden;
  }
  .mode {
    padding: 1px 6px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 600;
    white-space: nowrap;
  }
  .status {
    margin: 0;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
    font-size: 0.8rem;
  }
  .status.bad {
    color: var(--yellow);
  }
</style>
