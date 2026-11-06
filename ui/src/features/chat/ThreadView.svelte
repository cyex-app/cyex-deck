<!-- V109-C01 A reply thread in its own window, like the usercard: the message that started it
     and every reply to it, live, in the chat's own list (a ChatRoom that keeps only the
     thread's lines), so hover Reply, Pin (mods), the line menu, emotes and links work as in
     chat. Unpinned it closes on a click elsewhere; pin and resize from the frame. The
     "Follow" box makes every new line in the thread ping like a reply to you (Rust,
     `Room::follow_thread`; for this run). The message box at the bottom answers the first
     message by default (`defaultReply`), a line's Reply button answers that line instead, and
     clearing the reply bar goes back to the default. Route: `thread/<channel>/<first id>`. -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import Checkbox from '../../components/Checkbox.svelte';
  import Loading from '../../components/Loading.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { chatIpc } from '../../ipc/chat';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { dragCard } from '../../lib/window.svelte';
  import { catalogs } from './catalog.svelte';
  import ChatInput from './ChatInput.svelte';
  import MessageList from './MessageList.svelte';
  import { ChatRoom } from './room.svelte';
  import { defaultReply, inThread, parseThreadRoute } from './thread';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const { channel, threadId } = parseThreadRoute(route);

  const room = new ChatRoom(channel, (m) => inThread(m, threadId));
  const loaded = $derived(room.view > 0 || !!room.error);
  const replies = $derived(room.messages.filter((m) => m.id !== threadId).length);
  let following = $state(false);
  let busy = $state(false);
  let input = $state<ChatInput>();
  let replyTo = $state<ChatMessage | null>(null);

  // Nothing picked (or the bar cleared, or a send went out): answer the thread's first message.
  const fallback = $derived(defaultReply(room.messages, threadId));
  $effect(() => {
    if (!replyTo && fallback && room.account) replyTo = fallback;
  });

  void catalogs.loadGlobal();
  void catalogs.loadChannel(channel);

  onMount(() => {
    void room.open(accounts.current?.userId ?? null, [], true);
    chatIpc
      .threadFollowed(channel, threadId)
      .then((on) => (following = on))
      .catch(() => {});
  });
  onDestroy(() => room.close());

  async function toggle(on: boolean) {
    if (busy) return;
    busy = true;
    try {
      await chatIpc.threadFollow(channel, threadId, on);
      following = on;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function reply(id: string) {
    replyTo = room.find(id) ?? null;
  }

  /** The window moves from the bar's free space (`dragCard` skips controls). */
  const drag = (e: MouseEvent) => dragCard(e);
</script>

<div class="thread">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="bar" onmousedown={drag}>
    <span class="count">{replies ? `${formatNumber(replies)} ${replies === 1 ? 'reply' : 'replies'}` : 'Thread'}</span>
    <Checkbox
      checked={following}
      disabled={busy}
      label="Follow"
      hint="New replies in this thread ping you like a reply to you."
      onchange={(on) => void toggle(on)}
    />
  </div>
  <div class="body">
    {#if !room.messages.length}
      <div class="empty">{#if !loaded}<Loading />{:else}<p>{room.error || 'This thread has left the chat.'}</p>{/if}</div>
    {:else}
      <MessageList
        {room}
        thread={threadId}
        canReply={room.account !== null}
        onreply={reply}
        onmention={(l) => input?.mention(l)}
        onprefill={(t) => input?.prefill(t)}
        oninsert={(t) => input?.insertText(t)}
        onleave={() => input?.focus()}
      />
    {/if}
  </div>
  <ChatInput bind:this={input} {room} bind:replyTo onsignin={() => toast.error('Sign in from the main window.')} />
</div>

<style>
  .thread {
    container: chat / inline-size;
    position: relative;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--page);
    user-select: text;
  }
  .thread > :global(.input) {
    flex: none;
  }
  .bar {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .count {
    color: var(--muted);
    font-size: 0.78rem;
    font-weight: 700;
  }
  .body {
    position: relative;
    display: flex;
    flex: 1;
    min-height: 0;
  }
  .empty {
    flex: 1;
    padding: 40px 14px 0;
    color: var(--dim);
    font-size: 0.82rem;
    text-align: center;
  }
  .empty p {
    margin: 0;
  }
</style>
