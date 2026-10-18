<!-- 1.9 The channel's pinned message, above the chat. Can be folded to one line, hidden
     into the events button while it's pinned (V103-04) or closed until the pin changes;
     mods can take it down (V101-34). It always starts open, with who sent and who pinned
     it; folding is for this pin only (V106-19). A pin that ends shows a timer bar. Its text is semibold and a step larger than chat
     (Settings → General, V105-03), so it reads at a glance. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { tooltip } from '../../components/tooltip.svelte';
  import { reduced } from '../../lib/motion';
  import { settings } from '../../lib/settings.svelte';
  import Body from './Body.svelte';
  import { pinKey, pinLive, shows } from './eventStrip';
  import { eventStrip } from './eventStrip.svelte';
  import { shownName } from './names';
  import { pinSpan } from './pinTimer';

  interface Props {
    pin: Pinned;
    channel: string;
    /** Only for mods (1.57). */
    onunpin?: () => void;
  }

  let { pin, channel, onunpin }: Props = $props();

  /** V106-19 Open on every new pin; clicking folds it to two lines (one while the chat is
   * short, A13). Never saved. */
  let open = $state(true);
  let el = $state<HTMLElement>();
  let short = $state(false);

  /** A13 Under this the chat has little room for messages, so the banner takes one line. */
  const SHORT_PX = 300;
  $effect(() => {
    const host = el?.closest<HTMLElement>('.chat') ?? el?.parentElement;
    if (!host) return;
    const ro = new ResizeObserver(() => (short = host.clientHeight < SHORT_PX));
    ro.observe(host);
    return () => ro.disconnect();
  });

  // V106-19 A kept-mounted tab that shows again (display:none before) opens it again.
  $effect(() => {
    if (!el || typeof IntersectionObserver !== 'function') return;
    let was = false;
    const io = new IntersectionObserver(([entry]) => {
      if (entry.isIntersecting && !was) open = true;
      was = entry.isIntersecting;
    });
    io.observe(el);
    return () => io.disconnect();
  });

  const key = $derived(pinKey(pin.id));
  $effect(() => {
    key;
    open = true;
  });

  /** V106-19 Twitch's startsAt when it has one, else when this banner first saw the pin. */
  let seen = { id: '', at: 0 };
  function seenAt(id: string) {
    if (seen.id !== id) seen = { id, at: Date.now() };
    return seen.at;
  }
  let tick = $state(Date.now());
  $effect(() => {
    if (!reduced() || !pin.endsAt) return;
    const t = setInterval(() => (tick = Date.now()), 10_000);
    return () => clearInterval(t);
  });
  const span = $derived(pinSpan(pin, seenAt(pin.id), reduced() ? tick : Date.now()));
  const c = $derived(settings.v.chat);
  const sender = $derived(pin.message.sender);
  const name = $derived(sender ? shownName(sender, c.names, c.users[sender.id]) : null);
</script>

{#if shows(eventStrip.of(channel), key)}
  <div class="pin" class:paid={pin.paid} class:open class:short bind:this={el}>
    <Icon name="pin" size={14} />
    <div class="main">
      {#if (sender && name) || pin.by}
        <div class="head">
          {#if sender && name}
            <span
              class="name"
              style="--cd:{sender.color.dark};--cl:{sender.color.light}">{name.name}</span
            >
          {/if}
          {#if pin.by}<span class="by">{sender && name ? '· ' : ''}Pinned by {pin.by}</span>{/if}
        </div>
      {/if}
      <button class="text" onclick={() => (open = !open)} aria-expanded={open}>
        <Body fragments={pin.message.body} {channel} />
      </button>
    </div>
    {#if onunpin}
      <button class="close" aria-label="Unpin" use:tooltip={{ label: 'Unpin', hint: 'Takes it down for everyone.' }} onclick={onunpin}>
        <Icon name="trash" size={13} />
      </button>
    {/if}
    {#if pinLive(pin, Date.now())}
      <button
        class="close"
        aria-label="Hide pinned message"
        use:tooltip={{ label: 'Hide', hint: 'Tucks it into the events button.' }}
        onclick={() => eventStrip.hide(channel, { key, live: true })}
      >
        <Icon name="chevronUp" size={13} />
      </button>
    {/if}
    <button
      class="close"
      aria-label="Close pinned message"
      use:tooltip={{ label: 'Close', hint: 'Only for you, until the next pin.' }}
      onclick={() => eventStrip.close(channel, key)}
    >
      <Icon name="x" size={13} />
    </button>
    {#if span}
      {#key pin.id}
        <div
          class="timer"
          class:still={reduced()}
          style="--from:{span.frac};--dur:{Math.round(span.left)}ms"
          aria-hidden="true"
        ></div>
      {/key}
    {/if}
  </div>
{/if}

<style>
  .pin {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin: 6px 8px 2px;
    padding: 7px 8px 7px 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    position: relative;
    overflow: hidden;
  }
  .main {
    flex: 1;
    min-width: 0;
  }
  .head {
    margin-bottom: 2px;
    font-size: calc(var(--chat-size) * 0.9);
    line-height: 1.3;
  }
  /* V106-19 Shrinks to the pin's end on the compositor; no per-frame script. */
  .timer {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: var(--accent);
    opacity: 0.35;
    transform-origin: left center;
    transform: scaleX(var(--from));
    animation: pin-run var(--dur) linear forwards;
  }
  .timer.still {
    animation: none;
  }
  @keyframes pin-run {
    from {
      transform: scaleX(var(--from));
    }
    to {
      transform: scaleX(0);
    }
  }
  .pin.paid {
    border-color: var(--twitch);
  }
  .pin > :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .text {
    display: block;
    width: 100%;
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    text-align: left;
    /* V105-03 Tight lines; a line with an emote grows to fit it (below). */
    font: 400 calc(var(--chat-size) + var(--pinned-step)) / 1.3 var(--chat-font);
    overflow-wrap: anywhere;
  }
  .text :global(.emote),
  .text :global(.stack) {
    margin: 0;
    vertical-align: middle;
  }
  .pin:not(.open) .text {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .pin.short:not(.open) .text {
    -webkit-line-clamp: 1;
    line-clamp: 1;
  }
  .name {
    color: var(--cd);
    font-weight: 700;
  }
  :global(:root[data-theme='light']) .name {
    color: var(--cl);
  }
  .by {
    color: var(--dim);
    font-weight: 400;
  }
  .close {
    position: relative;
    display: grid;
    place-items: center;
    flex: none;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .close::before {
    content: "";
    position: absolute;
    inset: -1px;
  }
  .close:hover {
    color: var(--text);
  }
</style>
