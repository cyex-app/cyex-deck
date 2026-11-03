<script lang="ts">
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import { BASE, FAST, reduced } from '../lib/motion';
  import { cards } from '../lib/noticeCards.svelte';
  import { notifyEvents } from '../ipc/notify';
  import Avatar from './Avatar.svelte';
  import Icon from './Icon.svelte';

  // V108-C37 Listens here, so the main window draws them and nothing else does.
  $effect(() => {
    const off = notifyEvents.onCard((c) => cards.push(c));
    return () => void off.then((f) => f());
  });

  function act(id: number) {
    const c = cards.items.find((x) => x.id === id);
    cards.dismiss(id);
    if (c) void notifyEvents.open(c.open);
  }
</script>

<!-- Top right under the title bar, so the window buttons stay clear. Newest on top. Hover or
     focus holds a card. Separate from the centre toasts. -->
<div class="stack" role="region" aria-label="Notifications">
  {#each cards.items as c (c.id)}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="card {c.kind}"
      role="status"
      animate:flip={{ duration: reduced() ? 0 : BASE }}
      in:fly={{ x: 16, duration: reduced() ? 0 : BASE }}
      out:fly={{ x: 12, duration: reduced() ? 0 : FAST }}
      onmouseenter={() => cards.hold(c.id)}
      onmouseleave={() => cards.release(c.id)}
      onfocusin={() => cards.hold(c.id)}
      onfocusout={() => cards.release(c.id)}
    >
      <button class="body" onclick={() => act(c.id)}>
        <Avatar src={c.image} name={c.title} size={32} />
        <span class="words">
          <span class="title">{c.title}</span>
          {#if c.text}<span class="text">{c.text}</span>{/if}
        </span>
      </button>
      <button class="x" aria-label="Close" onclick={() => cards.dismiss(c.id)}><Icon name="x" size={10} /></button>
    </div>
  {/each}
</div>

<style>
  .stack {
    position: fixed;
    top: calc(var(--chrome-h) + 10px);
    right: 12px;
    width: min(320px, calc(100vw - 24px));
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
    z-index: var(--z-toast);
  }
  .card {
    position: relative;
    display: flex;
    align-items: stretch;
    background: color-mix(in srgb, var(--elevated) 82%, transparent);
    backdrop-filter: blur(14px) saturate(1.2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.85rem;
    pointer-events: auto;
  }
  .body {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 26px 9px 10px;
    background: none;
    border: 0;
    text-align: left;
  }
  .body:hover {
    background: var(--surface-hover);
  }
  .words {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .title {
    font-weight: 600;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .text {
    color: var(--muted);
    font-size: 0.8rem;
    /* One short line; the title or game under it is cut at the card's edge. */
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .x {
    position: absolute;
    top: 4px;
    right: 4px;
    width: 18px;
    height: 18px;
    display: grid;
    place-items: center;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--muted);
  }
  .x:hover {
    background: var(--surface-hover);
    color: inherit;
  }
  @media (prefers-reduced-transparency: reduce), (prefers-contrast: more) {
    .card {
      background: var(--elevated);
      backdrop-filter: none;
    }
  }
</style>
