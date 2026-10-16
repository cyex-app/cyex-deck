<!-- A channel as a list row (1.6): live ones with category, title and viewers, offline
     ones with when they were last live (H1). Hovering a live one peeks at it. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import { formatDate, formatShort } from '../../lib/format';
  import { cardClick, cardMenu } from './cardMenu';
  import { since, uptimeShort } from './lists';
  import { type Nav, picked } from './open.svelte';
  import { peekAt, unpeek, unpeekFrom } from './peeking.svelte';
  import QuickOpen from './QuickOpen.svelte';
  import { tooltip } from '../../components/tooltip.svelte';

  let { card, nav }: { card: ChannelCard; nav: Nav } = $props();

  const click = $derived(cardClick(nav, card));
  const stream = $derived(card.stream);
  const last = $derived(since(card.lastLive));
  // "Live 2 days ago · Just Chatting · title"; the full title rides in the tooltip.
  const offline = $derived([last ? `Live ${last}` : '', card.lastGame, card.lastTitle].filter(Boolean).join(' · '));
  const ownPeek = (node: HTMLElement) => ({ destroy: () => unpeekFrom(node) });
  const offlineTip = $derived([card.lastLive ? formatDate(new Date(card.lastLive)) : '', card.lastGame, card.lastTitle].filter(Boolean).join(' · '));
</script>

<div class="row" class:picked={picked.has(card.id)}>
  <button
    use:ownPeek
    class="main"
    class:live={!!stream}
    onclick={(e) => (unpeek(), click.onclick(e))}
    onauxclick={click.onauxclick}
    onmousedown={click.onmousedown}
    onmouseenter={(e) => stream && peekAt(e.currentTarget, card)}
    onmouseleave={unpeek}
    oncontextmenu={(e) => (e.preventDefault(), unpeek(), openMenu(e, cardMenu(nav, card)))}
  >
    <span class="av">
      <Avatar src={card.avatar} name={card.name} size={26} />
      {#if stream}<span class="dot" class:rerun={stream.rerun}></span>{/if}
    </span>
    <span class="name">{card.name}</span>
    {#if stream}
      <span class="game">{stream.game?.name ?? ''}</span>
      <span class="title">{stream.title}</span>
      <span class="num">{formatShort(stream.viewers)}</span>
      <span class="num dim">{uptimeShort(stream.startedAt)}</span>
    {:else}
      <span class="title dim" use:tooltip={{ label: offlineTip, when: () => !!offlineTip }}>{offline}</span>
    {/if}
  </button>
  <div class="quick"><QuickOpen channel={card} {nav} flat /></div>
</div>

<style>
  .row {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius);
  }
  .row:hover {
    background: var(--row-hover);
  }
  .picked {
    background: var(--accent-dim);
  }
  .main {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: 30px minmax(90px, 170px) 1fr;
    align-items: center;
    gap: 10px;
    height: 36px;
    padding: 0 8px;
    background: none;
    border: 0;
    color: var(--text);
    text-align: left;
  }
  .main:not(.live) {
    grid-template-columns: 30px max-content 1fr;
  }
  .main.live {
    grid-template-columns: 30px minmax(90px, 170px) minmax(60px, 160px) 1fr 52px 64px;
  }
  .av {
    position: relative;
    display: flex;
  }
  .dot {
    position: absolute;
    right: -1px;
    bottom: -1px;
    width: 9px;
    height: 9px;
    background: var(--live);
    border: 2px solid var(--bg);
    border-radius: 50%;
  }
  .dot.rerun {
    background: var(--yellow);
  }
  .name {
    font-weight: 600;
    font-size: 0.85rem;
  }
  .name,
  .game,
  .title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .game,
  .title {
    font-size: 0.8rem;
    color: var(--muted);
  }
  .num {
    text-align: right;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .dim {
    color: var(--dim);
  }
  .quick {
    position: absolute;
    right: 4px;
    padding-left: 12px;
    background: linear-gradient(to right, transparent, var(--bg) 12px);
    opacity: 0;
    pointer-events: none;
  }
  .row:hover .quick,
  .quick:focus-within {
    opacity: 1;
    pointer-events: auto;
  }
</style>
