<script lang="ts">
  import { untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import { BASE, FAST, reduced } from '../lib/motion';
  import { cards } from '../lib/noticeCards.svelte';
  import { streamer } from '../lib/streamer.svelte';
  import { notifyEvents } from '../ipc/notify';
  import { tabs, tiles } from '../features/tabs/tabs.svelte';
  import Avatar from './Avatar.svelte';
  import Button from './Button.svelte';
  import { type CardAction, cardAction } from './noticeActions';
  import Icon from './Icon.svelte';

  // V108-C37 Listens here, so the main window draws them and nothing else does.
  $effect(() => {
    const off = notifyEvents.onCard((c) => cards.push(c));
    return () => void off.then((f) => f());
  });

  // V108-C37e Streamer mode coming on takes the cards already up off the screen too.
  $effect(() => {
    if (streamer.hides('noNotifications')) untrack(() => cards.items.map((c) => c.id)).forEach((id) => cards.dismiss(id));
  });

  // V108-C39 Read at draw time, so the button is right if a tab opened or closed meanwhile.
  const tabRefs = $derived(tabs.list.map((t) => ({ id: t.id, logins: tiles(t).map((x) => x.channel.login) })));

  function press(id: number, a: CardAction) {
    cards.dismiss(id);
    if (a.raise) void notifyEvents.focusMain();
    void notifyEvents.open(a.open);
  }

  function act(id: number) {
    const c = cards.items.find((x) => x.id === id);
    cards.dismiss(id);
    if (c) void notifyEvents.open(c.open);
  }
</script>

<!-- Top right under the title bar, so the window buttons stay clear. Newest on top. Hover or
     focus holds a card; C37d it lets go only when neither the pointer nor focus is on it. Separate from the centre toasts. -->
<div class="stack" role="region" aria-label="Notifications">
  {#each cards.items as c (c.id)}
    {@const a = cardAction(c.kind, c.open, tabRefs)}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions (hover and focus hold it; Escape from its buttons closes it) -->
    <div
      class="card {c.kind}"
      role="status"
      animate:flip={{ duration: reduced() ? 0 : BASE }}
      in:fly={{ x: 16, duration: reduced() ? 0 : BASE }}
      out:fly={{ x: 12, duration: reduced() ? 0 : FAST }}
      onmouseenter={() => cards.hold(c.id)}
      onmouseleave={(e) => !e.currentTarget.contains(document.activeElement) && cards.release(c.id)}
      onfocusin={() => cards.hold(c.id)}
      onfocusout={(e) => !e.currentTarget.matches(':hover') && !e.currentTarget.contains(e.relatedTarget as Node | null) && cards.release(c.id)}
      onkeydown={(e) => e.key === 'Escape' && cards.dismiss(c.id)}
    >
      <button class="body" title={c.text || undefined} onclick={() => act(c.id)}>
        <Avatar src={c.image} name={c.title} size={32} />
        <span class="words">
          <span class="title">{c.title}</span>
          <!-- V108-C37e One line each: the go-live message (or title), then the game. -->
          {#each c.text.split('\n').filter(Boolean).slice(0, 2) as line, i (i)}<span class="text">{line}</span>{/each}
        </span>
      </button>
      <button class="x" aria-label="Close" onclick={() => cards.dismiss(c.id)}><Icon name="x" size={10} /></button>
      {#if a}
        <div class="actions"><Button small onclick={() => press(c.id, a)}>{a.label}</Button></div>
      {/if}
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
    flex-direction: column;
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
  /* V108-C39 Under the text, level with it; a sibling of the body so a press never opens the card. */
  .actions {
    display: flex;
    padding: 0 10px 9px 52px;
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
