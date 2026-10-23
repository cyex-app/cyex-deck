<!-- A live channel as a card (1.6, 1.7): preview picture with viewers and uptime, then
     name, title and category. Hover shows the quick options. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import type { GameRef } from '../../ipc/bindings/GameRef';
  import { formatShort } from '../../lib/format';
  import { cardClick, cardMenu } from './cardMenu';
  import { preview, uptimeShort } from './lists';
  import { type Nav, picked } from './open.svelte';
  import QuickOpen from './QuickOpen.svelte';

  interface Props {
    card: ChannelCard;
    nav: Nav;
    /** H4 */
    oncategory?: (game: GameRef) => void;
    ontag?: (tag: string) => void;
    /** Inside a category, its name is left out. */
    showGame?: boolean;
  }

  let { card, nav, oncategory, ontag, showGame = true }: Props = $props();

  const stream = $derived(card.stream!);
  const click = $derived(cardClick(nav, card));
  const started = $derived(Date.parse(stream.startedAt));
</script>

<div class="card" class:picked={picked.has(card.id)}>
  <button
    class="pic"
    aria-label="Open {card.name}"
    onclick={click.onclick}
    onauxclick={click.onauxclick}
    onmousedown={click.onmousedown}
    oncontextmenu={(e) => (e.preventDefault(), openMenu(e, cardMenu(nav, card)))}
  >
    <img src={preview(card.login)} alt="" loading="lazy" decoding="async" />
    {#if stream.rerun}<span class="tag rerun">Rerun</span>{/if}
    <span class="tag viewers">{formatShort(stream.viewers)} {stream.viewers === 1 ? 'viewer' : 'viewers'}</span>
    {#if started}<span class="tag uptime">{uptimeShort(stream.startedAt)}</span>{/if}
  </button>
  <div class="quick"><QuickOpen channel={card} {nav} /></div>
  <div class="info">
    <Avatar src={card.avatar} name={card.name} size={32} />
    <div class="text">
      <span class="name">{card.name}</span>
      <span class="title" title={stream.title}>{stream.title || 'Untitled'}</span>
      {#if showGame && stream.game}
        {#if oncategory}
          <button class="game link" onclick={() => oncategory(stream.game!)}>{stream.game.name}</button>
        {:else}
          <span class="game">{stream.game.name}</span>
        {/if}
      {/if}
      {#if ontag && stream.tags.length}
        <div class="tags">
          {#each stream.tags.slice(0, 3) as t (t)}
            <button class="chip" onclick={() => ontag(t)}>{t}</button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .card {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }
  .pic {
    position: relative;
    aspect-ratio: 16 / 9;
    padding: 0;
    background: var(--surface);
    border: 0;
    border-radius: var(--radius);
    overflow: hidden;
  }
  .picked .pic {
    outline: 2px solid var(--accent-line);
    outline-offset: 2px;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    transition: transform var(--base) var(--ease);
  }
  .pic:hover img {
    transform: scale(1.03);
  }
  .tag {
    position: absolute;
    padding: 0 5px;
    background: var(--scrim-strong);
    border-radius: var(--radius);
    color: var(--on-video);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
  }
  .rerun {
    top: 6px;
    left: 6px;
    background: #9a6a00;
    font-weight: 700;
  }
  .viewers {
    bottom: 6px;
    left: 6px;
  }
  .uptime {
    bottom: 6px;
    right: 6px;
  }
  .quick {
    position: absolute;
    top: 6px;
    right: 6px;
    opacity: 0;
    transition: opacity var(--fast);
  }
  .card:hover .quick,
  .quick:focus-within {
    opacity: 1;
  }
  .info {
    display: flex;
    gap: 10px;
    min-width: 0;
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    font-size: 0.85rem;
  }
  .title {
    font-size: 0.8rem;
    color: var(--muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .game {
    align-self: flex-start;
    max-width: 100%;
    font-size: 0.78rem;
    color: var(--dim);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .link {
    padding: 0;
    background: none;
    border: 0;
    text-align: left;
  }
  .link:hover {
    color: var(--accent-text);
  }
  .tags {
    display: flex;
    gap: 4px;
    margin-top: 2px;
    overflow: hidden;
  }
  .chip {
    flex: none;
    padding: 0 7px;
    height: 20px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-pill);
    color: var(--muted);
    font-size: 0.72rem;
  }
  .chip:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
</style>
