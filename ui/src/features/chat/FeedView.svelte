<!-- F4 A feed tab: messages that highlight rules send here (“Custom feed”), from every
     joined chat. Read only; names and links open like in a chat. The Mentions tab (V104-01)
     is one too: global mentions (V104-02) are marked, and a channel opens its own small
     chat window (V102-05). -->
<script lang="ts">
  import { onDestroy, tick, type Snippet } from 'svelte';

  import { ClearUndo } from '../feeds/clearUndo.svelte';
  import FeedHeader from '../feeds/FeedHeader.svelte';
  import Button from '../../components/Button.svelte';
  import type { FeedEvent } from '../../ipc/bindings/FeedEvent';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { popOut } from '../../app/panels';
  import { chatxIpc } from '../../ipc/chatx';
  import { settings } from '../../lib/settings.svelte';
  import { lineClick } from './clicks';
  import Line from './Line.svelte';
  import { isMentionsFeed, whenText } from './mentionsTab';
  import { roomRoute } from './roomRoute';
  import { tooltip } from '../../components/tooltip.svelte';

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

  async function apply(events: FeedEvent[]) {
    const follow = atBottom();
    let next = lines;
    for (const e of events) {
      if (e.type === 'line') next = [...next, e.line].slice(-KEEP);
      else if (e.type === 'remove') next = next.filter((l) => !e.keys.includes(lineKey(l)));
      else next = next.map((l) => (l.message.id === e.line.message.id ? e.line : l));
    }
    lines = next;
    if (follow) {
      await tick();
      list?.scrollTo({ top: list.scrollHeight });
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
  const stamped = $derived(settings.v.chat.timestamps.show);

  function openRoom(channel: string) {
    void popOut(roomRoute(channel, null), `${channel} chat`);
  }
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
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="lines" bind:this={list} onclick={lineClick}>
    {#each shown as l (`${l.channel}/${l.message.id}`)}
      <div data-channel={l.channel}>
        <div class="chan">
          <button class="room" use:tooltip={'Open this chat'} onclick={(e) => (e.stopPropagation(), openRoom(l.channel))}>
            #{l.channel}
          </button>
          {#if l.global}<span class="global" use:tooltip={'From logs.cyex.app'}>global</span>{/if}
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
        {:else}
          Nothing yet. Highlight rules with “{name}” as their custom feed send messages here.
        {/if}
      </p>
    {/each}
  </div>
</section>

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
  .when {
    margin-left: auto;
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
