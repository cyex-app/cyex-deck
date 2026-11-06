<!-- V109-C01 A reply thread in its own window, like the usercard: the message that started it
     and every reply to it, live. Unpinned it closes on a click elsewhere; pin and resize from
     the frame. "Follow" makes every new line in the thread ping like a reply to you (Rust,
     `Room::follow_thread`; for this run). The thread is read from a view of the chat that
     is open, so deletes and edits arrive as in the chat. Route: `thread/<channel>/<first id>`. -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import Button from '../../components/Button.svelte';
  import Loading from '../../components/Loading.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { chatIpc } from '../../ipc/chat';
  import { accounts } from '../../lib/accounts.svelte';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { dragCard } from '../../lib/window.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { catalogs } from './catalog.svelte';
  import Line from './Line.svelte';
  import { emoteMenu, enterClicks, lineTips, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
  import { ChatRoom } from './room.svelte';
  import { collect, orphanRoot, parseThreadRoute } from './thread';

  let { route }: { route: string; popout?: boolean } = $props();

  // The route never changes for a window.
  // svelte-ignore state_referenced_locally
  const { channel, threadId } = parseThreadRoute(route);

  const room = new ChatRoom(channel);
  const lines = $derived(collect(room.messages, threadId));
  const orphan = $derived(orphanRoot(room.messages, threadId));
  const loaded = $derived(room.view > 0 || !!room.error);
  const replies = $derived(lines.filter((m) => m.id !== threadId).length);
  let following = $state(false);
  let busy = $state(false);
  let card = $state<CardTarget | null>(null);
  let box = $state<HTMLElement>();
  let inner = $state<HTMLElement>();
  let atBottom = true;

  void catalogs.loadGlobal();
  void catalogs.loadChannel(channel);

  const account = $derived(room.account ?? accounts.current?.userId ?? null);
  const host = (): LineHost => ({ channel, account, senderOf: () => undefined, openCard: (t) => (card = t), openMenu });
  const tips = lineTips(() => inner);

  onMount(() => {
    void room.open(accounts.current?.userId ?? null, [], true);
    chatIpc
      .threadFollowed(channel, threadId)
      .then((on) => (following = on))
      .catch(() => {});
  });
  onDestroy(() => {
    tips.drop();
    room.close();
  });

  async function toggle() {
    if (busy) return;
    busy = true;
    const on = !following;
    try {
      await chatIpc.threadFollow(channel, threadId, on);
      following = on;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(PARTS);
    if (el) openPart(e, el, host());
  }

  function oncontextmenu(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-emote]');
    if (el) void emoteMenu(e, el, host());
    else notEmote();
  }

  function onscroll() {
    if (!box) return;
    tips.drop();
    atBottom = box.scrollHeight - box.scrollTop - box.clientHeight < 24;
  }

  // Starts at the newest reply and stays there while new ones come in and pictures load.
  $effect(() => {
    if (!box || !inner) return;
    const list = box;
    const ro = new ResizeObserver(() => {
      if (atBottom) list.scrollTop = list.scrollHeight;
    });
    ro.observe(inner);
    return () => ro.disconnect();
  });

  /** The window moves from anywhere that isn't a control or the scroll list (`dragCard`). */
  const drag = (e: MouseEvent) => dragCard(e, '.list');
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="thread" onmousedown={drag}>
  <div class="bar">
    <span class="count">{replies ? `${formatNumber(replies)} ${replies === 1 ? 'reply' : 'replies'}` : 'Thread'}</span>
    <Button
      small
      variant={following ? 'secondary' : 'primary'}
      icon={following ? 'bellOff' : 'bell'}
      disabled={busy}
      onclick={() => void toggle()}>{following ? 'Unfollow' : 'Follow'}</Button
    >
  </div>
  <!-- Clicks, keys and hovers are delegated from the lines' emotes, names and links. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
  <div
    class="list"
    bind:this={box}
    {onscroll}
    {onclick}
    {oncontextmenu}
    onkeydown={enterClicks}
    onmouseover={tips.onmouseover}
    onmouseout={tips.onmouseout}
  >
    <div bind:this={inner}>
      {#if orphan}
        <p class="orphan"><b>{orphan.display}</b> <bdi>{orphan.body}</bdi></p>
      {/if}
      {#each lines as m (m.id)}<Line {m} {channel} replyHidden={lines[0]?.id === threadId ? threadId : undefined} />{/each}
      {#if !lines.length && !orphan}
        {#if !loaded}<Loading />{:else}<p class="empty">{room.error || 'This thread has left the chat.'}</p>{/if}
      {/if}
      {#if following}<p class="note">New replies ping you.</p>{/if}
    </div>
  </div>
</div>
{#if card}
  {#key card}<EmoteCard target={card} login={channel} {account} onclose={() => (card = null)} />{/key}
{/if}

<style>
  .thread {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    user-select: text;
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
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: var(--page);
    padding-bottom: 4px;
  }
  /* The reply row names the line it answers; here that's the thread itself, nothing to jump to. */
  .list :global(.context[data-jump]) {
    cursor: default;
  }
  .list :global(.context[data-jump]:hover) {
    color: inherit;
  }
  /* The first line when it has left the chat: what the reply tags carry, dimmed. */
  .orphan {
    margin: 0;
    padding: 4px 10px;
    color: var(--dim);
    font-size: 0.87rem;
    overflow-wrap: anywhere;
  }
  .orphan b {
    color: var(--muted);
  }
  .note,
  .empty {
    margin: 0;
    padding: 8px 14px;
    color: var(--dim);
    font-size: 0.78rem;
  }
  .empty {
    padding-top: 40px;
    text-align: center;
    font-size: 0.82rem;
  }
</style>
