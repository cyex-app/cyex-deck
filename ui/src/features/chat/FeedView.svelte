<!-- F4 A feed tab: messages that highlight rules send here ("Also show in"), from every
     joined chat. Read only; names and links open like in a chat. -->
<script lang="ts">
  import { onDestroy, tick, type Snippet } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import type { FeedEvent } from '../../ipc/bindings/FeedEvent';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { chatxIpc } from '../../ipc/chatx';
  import { lineClick } from './clicks';
  import Line from './Line.svelte';

  interface Props {
    name: string;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { name, actions, onheaderdown }: Props = $props();

  const KEEP = 500;
  let lines = $state.raw<FeedLine[]>([]);
  let list = $state<HTMLElement>();
  let view = 0;
  let closed = false;

  function atBottom() {
    return !list || list.scrollHeight - list.scrollTop - list.clientHeight < 40;
  }

  async function apply(events: FeedEvent[]) {
    const follow = atBottom();
    let next = lines;
    for (const e of events) {
      if (e.type === 'line') next = [...next, e.line].slice(-KEEP);
      else next = next.map((l) => (l.message.id === e.line.message.id ? e.line : l));
    }
    lines = next;
    if (follow) {
      await tick();
      list?.scrollTo({ top: list.scrollHeight });
    }
  }

  // svelte-ignore state_referenced_locally
  chatxIpc
    .feedOpen(name, (e) => void apply(e))
    .then((o) => {
      if (closed) return void chatxIpc.feedClose(o.view);
      view = o.view;
      void apply(o.lines.map((line) => ({ type: 'line', line })));
    })
    .catch(() => {});

  onDestroy(() => {
    closed = true;
    if (view) void chatxIpc.feedClose(view);
  });
</script>

<section class="feed">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={onheaderdown}>
    <Icon name="list" size={14} />
    <span class="name">{name}</span>
    {@render actions?.()}
  </header>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="lines" bind:this={list} onclick={lineClick}>
    {#each lines as l (`${l.channel}/${l.message.id}`)}
      <div data-channel={l.channel}>
        <div class="chan">#{l.channel}</div>
        <Line m={l.message} channel={l.channel} />
      </div>
    {:else}
      <p class="empty">Nothing yet. Messages land here when a highlight rule says “Also show in {name}”.</p>
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
    background: var(--bg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 6px 0 10px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  .name {
    flex: 1;
    color: var(--text);
    font-weight: 700;
  }
  .lines {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .chan {
    padding: 4px 10px 0;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .empty {
    padding: 10px 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
