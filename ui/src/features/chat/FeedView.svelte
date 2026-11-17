<!-- F4 A feed tab: messages that highlight rules send here (“Custom feed”), from every
     joined chat. Read only; names and links open like in a chat. The Mentions tab (V104-01)
     is one too: global mentions (V104-02) are marked, and a channel opens its own small
     chat window (V102-05). V111-53 Mention tabs list the newest line at the top; each line
     has Show in logs, and Show in chat while its chat here still has it. -->
<script lang="ts">
  import { onDestroy, tick, type Snippet } from 'svelte';

  import { ClearUndo } from '../feeds/clearUndo.svelte';
  import FeedHeader from '../feeds/FeedHeader.svelte';
  import Button from '../../components/Button.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import type { FeedEvent } from '../../ipc/bindings/FeedEvent';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { popOut } from '../../app/panels';
  import { chatxIpc } from '../../ipc/chatx';
  import { settings } from '../../lib/settings.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { accounts } from '../../lib/accounts.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { FEED_CHAN, feedEvents } from './feedLines';
  import { lineTips } from './lineEvents';
  import Line from './Line.svelte';
  import { GLOBAL_FEED, feedKey, isMentionsFeed, whenText } from './mentionsTab';
  import { roomRoute } from './roomRoute';
  import { tooltip } from '../../components/tooltip.svelte';
  import { currentWindow } from '../../lib/window.svelte';
  import { tabs } from '../tabs/tabs.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { chatJump } from './chatJump.svelte';
  import { chatSpot, inChatCheck, newestFirst, ordered, type ChatSpot } from './feedJump';
  import { jumpInLogs } from './inLogs';

  interface Props {
    name: string;
    /** Header text; `name` stays the key Rust routes by. */
    title?: string;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { name, title, actions, onheaderdown }: Props = $props();

  const KEEP = 500;
  let lines = $state.raw<FeedLine[]>([]);
  let list = $state<HTMLElement>();
  let view = 0;
  let closed = false;

  function atBottom() {
    return !list || list.scrollHeight - list.scrollTop - list.clientHeight < 40;
  }

  const lineKey = (l: FeedLine) => `${l.channel}/${l.message.id}`;

  /** V107-MF Clear with Undo, as in the kind feeds. */
  const undo = new ClearUndo(
    () => lines.map(lineKey),
    (keys) => chatxIpc.feedClear(name, keys),
    (gone) => (lines = lines.filter((l) => !gone.has(lineKey(l)))),
  );
  const shown = $derived(undo.hidden ? lines.filter((l) => undo.shows(lineKey(l))) : lines);
  const newest = $derived(newestFirst(name));
  const drawn = $derived(ordered(shown, newest));

  /** V111-53 Newest first: the row at the top of the view, to keep in place while lines come in
   *  above it. None at the very top, where new lines are meant to show. The browser's scroll
   *  anchoring also stays on: it covers rows that size in after the tick. */
  function topRow(): { el: HTMLElement; off: number } | null {
    if (!list || list.scrollTop < 2) return null;
    const rows = list.children;
    let lo = 0;
    let hi = rows.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      const el = rows[mid] as HTMLElement;
      if (el.offsetTop + el.offsetHeight <= list.scrollTop) lo = mid + 1;
      else hi = mid;
    }
    const el = rows[lo] as HTMLElement | undefined;
    return el ? { el, off: el.offsetTop - list.scrollTop } : null;
  }

  async function apply(events: FeedEvent[]) {
    const follow = !newest && atBottom();
    const keep = newest ? topRow() : null;
    let next = lines;
    for (const e of events) {
      if (e.type === 'line') next = [...next, e.line].slice(-KEEP);
      else if (e.type === 'remove') {
        const gone = new Set(e.keys);
        next = next.filter((l) => !gone.has(lineKey(l)));
      } else next = next.map((l) => (l.message.id === e.line.message.id ? e.line : l));
    }
    lines = next;
    if (follow) {
      await tick();
      list?.scrollTo({ top: list.scrollHeight });
    } else if (keep) {
      await tick();
      if (list && keep.el.isConnected) list.scrollTop = keep.el.offsetTop - keep.off;
    }
  }

  let failed = $state(false);

  function open() {
    failed = false;
    chatxIpc
      .feedOpen(name, (e) => void apply(e))
      .then((o) => {
        if (closed) return void chatxIpc.feedClose(o.view);
        view = o.view;
        void apply(o.lines.map((line) => ({ type: 'line', line })));
      })
      .catch(() => !closed && (failed = true));
  }
  // svelte-ignore state_referenced_locally
  open();

  onDestroy(() => {
    undo.flush();
    closed = true;
    if (view) void chatxIpc.feedClose(view);
  });

  const mentions = $derived(isMentionsFeed(name));
  const globalFeed = $derived(feedKey(name) === feedKey(GLOBAL_FEED));
  const stamped = $derived(settings.v.chat.timestamps.show);

  function openRoom(channel: string) {
    void popOut(roomRoute(channel, null), `${channel} chat`);
  }

  /** V111-53 Show in chat: a chat tile of the channel in this window whose room still has the line. */
  const inChat = inChatCheck(
    (c) =>
      chatSpot(tabs.list, c, {
        active: tabs.active,
        popped: tabs.popped,
        poppedChats: tabs.chats,
        only: currentWindow.isMain ? null : (list?.closest('[data-tab-view]')?.getAttribute('data-tab-view') ?? ''),
      }),
    chatxIpc.hasLine,
  );
  const showIn = (spot: ChatSpot, id: string) => () => chatJump.go(spot, id);
  async function inChatRun(channel: string, m: ChatMessage) {
    const spot = await inChat(channel, m.id);
    return spot ? showIn(spot, m.id) : null;
  }
  /** The row the pointer or focus is on, when its chat still has the line. */
  let here = $state.raw<{ key: string; spot: ChatSpot } | null>(null);
  function probe(l: FeedLine) {
    const key = lineKey(l);
    void inChat(l.channel, l.message.id).then((spot) => {
      if (spot) here = { key, spot };
      else if (here?.key === key) here = null;
    });
  }
  const logsOf = (l: FeedLine) => (l.message.sender && !l.message.notice ? jumpInLogs(l.channel, l.message)[0] : undefined);

  /** The emote card on screen, with the channel of the line it came from. */
  let card = $state<{ target: CardTarget; channel: string } | null>(null);
  const events = feedEvents({
    find: (channel, id) => lines.find((l) => l.channel === channel && l.message.id === id)?.message,
    openCard: (target) => (card = { target, channel: lastChannel }),
    openMenu,
    openRoom,
    inChat: inChatRun,
  });
  let lastChannel = '';
  const chanOf = (e: Event) =>
    (e.target as HTMLElement).closest(`[${FEED_CHAN}]`)?.getAttribute(FEED_CHAN) ?? '';
  const tips = lineTips(() => list);
  onDestroy(tips.drop);
</script>

<section class="feed">
  <FeedHeader
    icon="list"
    name={title ?? name}
    hint={title && title !== name ? `Highlight rules send here as “${name}”.` : undefined}
    {actions}
    {onheaderdown}
  >
    {#snippet extra()}
      {#if shown.length > 0}
        <Button small variant="ghost" onclick={undo.clear}>Clear</Button>
      {/if}
    {/snippet}
  </FeedHeader>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
  <div
    class="lines"
    bind:this={list}
    onclick={(e) => ((lastChannel = chanOf(e)), events.onclick(e))}
    oncontextmenu={(e) => ((lastChannel = chanOf(e)), events.oncontextmenu(e))}
    onmouseover={tips.onmouseover}
    onmouseout={tips.onmouseout}
  >
    {#each drawn as l (`${l.channel}/${l.message.id}`)}
      {@const logs = logsOf(l)}
      <div class="row" {...{ [FEED_CHAN]: l.channel }} onpointerenter={() => probe(l)} onfocusin={() => probe(l)}>
        <div class="chan">
          <button class="room" use:tooltip={'Open this chat'} onclick={(e) => (e.stopPropagation(), openRoom(l.channel))}>
            #{l.channel}
          </button>
          {#if l.global}<span class="global" use:tooltip={'From logs.cyex.app'}>global</span>{/if}
          <span class="acts">
            {#if here?.key === lineKey(l)}
              {@const spot = here.spot}
              <IconButton icon="chat" label="Show in chat" size={13} onclick={(e) => (e.stopPropagation(), showIn(spot, l.message.id)())} />
            {/if}
            {#if logs}<IconButton icon="scroll" label="Show in logs" size={13} onclick={(e) => (e.stopPropagation(), logs.run())} />{/if}
          </span>
          <span class="when">{whenText(l.message.ts, stamped)}</span>
        </div>
        <Line m={l.message} channel={l.channel} />
      </div>
    {:else}
      <p class="empty">
        {#if failed}
          Couldn’t load this feed. <button class="retry" onclick={open}>Retry</button>
        {:else if mentions}
          Nothing yet. Your mentions, and highlights with “Mentions tab” on, land here.
        {:else if globalFeed}
          Nothing yet. When someone mentions you in a chat you aren’t in, it shows up here.
        {:else}
          Nothing yet. Highlight rules with “{name}” as their custom feed send messages here.
        {/if}
      </p>
    {/each}
  </div>
</section>
{#if card}
  {#key card}<EmoteCard target={card.target} login={card.channel} account={accounts.current?.userId ?? null} onclose={() => (card = null)} />{/key}
{/if}

<style>
  .feed {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--page);
  }
  .lines {
    position: relative;
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .chan {
    display: flex;
    align-items: baseline;
    gap: 6px;
    padding: 4px 10px 0;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .room {
    padding: 0;
    background: none;
    border: 0;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }
  .room:hover {
    color: var(--text);
    text-decoration: underline;
  }
  .global {
    padding: 0 4px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    font-weight: 600;
  }
  /* V111-53 Row actions: shown on hover or focus, in the gap before the time. */
  .acts {
    margin-left: auto;
    display: flex;
    align-self: center;
    gap: 2px;
    opacity: 0;
    transition: opacity var(--fast);
  }
  .row:hover .acts,
  .row:focus-within .acts {
    opacity: 1;
  }
  .acts :global(.icon-btn) {
    width: 22px;
    height: 18px;
    margin: -3px 0;
  }
  .when {
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .empty {
    padding: 10px 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .retry {
    background: none;
    border: 0;
    color: var(--text);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
