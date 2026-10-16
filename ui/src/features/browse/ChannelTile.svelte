<!-- An offline channel in home's grid (1.6): picture, name, when it was last live (H1). -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import { formatDate } from '../../lib/format';
  import { cardClick, cardMenu } from './cardMenu';
  import { since } from './lists';
  import { type Nav, picked } from './open.svelte';
  import QuickOpen from './QuickOpen.svelte';
  import { tooltip } from '../../components/tooltip.svelte';

  let { card, nav }: { card: ChannelCard; nav: Nav } = $props();

  const click = $derived(cardClick(nav, card));
  const last = $derived(since(card.lastLive));
  const sub = $derived([card.lastGame, card.lastTitle].filter(Boolean).join(' · '));
</script>

<div class="tile" class:picked={picked.has(card.id)}>
  <button
    class="body"
    onclick={click.onclick}
    onauxclick={click.onauxclick}
    onmousedown={click.onmousedown}
    oncontextmenu={(e) => (e.preventDefault(), openMenu(e, cardMenu(nav, card)))}
  >
    <Avatar src={card.avatar} name={card.name} size={52} />
    <span class="name">{card.name}</span>
    <span class="last" use:tooltip={{ label: card.lastLive ? formatDate(new Date(card.lastLive)) : '', when: () => !!card.lastLive }}>
      {last ? `Live ${last}` : 'Not live lately'}
    </span>
    {#if sub}<span class="sub" use:tooltip={{ label: sub }}>{sub}</span>{/if}
  </button>
  <div class="quick"><QuickOpen channel={card} {nav} flat /></div>
</div>

<style>
  .tile {
    position: relative;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .tile:hover {
    background: var(--surface-hover);
  }
  .picked {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
  }
  .body {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 14px 8px 36px;
    background: none;
    border: 0;
    color: var(--text);
    min-width: 0;
  }
  .name {
    max-width: 100%;
    margin-top: 4px;
    font-weight: 600;
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .last {
    color: var(--dim);
    font-size: 0.75rem;
  }
  .sub {
    max-width: 100%;
    color: var(--dim);
    font-size: 0.7rem;
    opacity: 0.8;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .quick {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 4px;
    display: flex;
    justify-content: center;
    opacity: 0;
    transition: opacity var(--fast);
  }
  .tile:hover .quick,
  .quick:focus-within {
    opacity: 1;
  }
</style>
