<!-- 1.9 The channel's pinned message, above the chat. Can be folded to one line, hidden
     into the events button while it's pinned (V103-04) or closed until the pin changes;
     mods can take it down (V101-34). It always starts open, with who sent and who pinned
     it; folding is for this pin only (V106-19). A pin that ends shows a timer bar. Its text is semibold and a step larger than chat
     (Settings → General, V105-03), so it reads at a glance. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { hideTip, showTip, tooltip } from '../../components/tooltip.svelte';
  import { systemIpc } from '../../ipc/system';
  import { errorText } from '../../lib/errors';
  import { reduced } from '../../lib/motion';
  import { toast } from '../../lib/toast.svelte';
  import { settings } from '../../lib/settings.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import { emojiName } from '../emotes/emoji';
  import type { CardTarget } from '../emotes/emotes';
  import Body from './Body.svelte';
  import { pinKey, pinLive, shows } from './eventStrip';
  import { eventStrip } from './eventStrip.svelte';
  import { shownName } from './names';
  import { pinSpan } from './pinTimer';

  interface Props {
    pin: Pinned;
    channel: string;
    /** V106-31/32 The account reading the chat, for the emote card's 7TV edits. */
    account?: string | null;
    /** Only for mods (1.57). */
    onunpin?: () => void;
  }

  let { pin, channel, account = null, onunpin }: Props = $props();

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
  // s1-1 Reduce motion can flip while a pin shows: the bar and its tick follow at once.
  let still = $state(reduced());
  $effect(() => {
    if (typeof matchMedia !== 'function') return;
    const q = matchMedia('(prefers-reduced-motion: reduce)');
    const flip = () => (still = q.matches);
    q.addEventListener('change', flip);
    return () => q.removeEventListener('change', flip);
  });
  let tick = $state(Date.now());
  $effect(() => {
    if (!still || !pin.endsAt) return;
    tick = Date.now();
    const t = setInterval(() => (tick = Date.now()), 10_000);
    return () => clearInterval(t);
  });
  const span = $derived(pinSpan(pin, seenAt(pin.id), still ? tick : Date.now()));

  /** s3-1 Unpin takes it down for everyone: a second click within 3 s does it. */
  let armed = $state(false);
  let armTimer: ReturnType<typeof setTimeout> | undefined;
  function unpin() {
    clearTimeout(armTimer);
    if (!armed) {
      armed = true;
      armTimer = setTimeout(() => (armed = false), 3000);
      return;
    }
    armed = false;
    onunpin?.();
  }
  $effect(() => () => clearTimeout(armTimer));

  /** 1.41 The emote card opened from the pin, like from a chat line. */
  let card = $state<CardTarget | null>(null);

  /** c-9 A link in the text opens, an emote opens its card; the rest of the text folds. */
  function textClick(e: MouseEvent) {
    const emote = (e.target as HTMLElement).closest<HTMLElement>('[data-emote]');
    if (emote) {
      e.stopPropagation();
      hideTip();
      const d = JSON.parse(emote.dataset.emote!) as Omit<CardTarget, 'rect'>;
      const sender = pin.message.sender?.id;
      card = { ...d, ...(sender && { sender }), rect: emote.getBoundingClientRect() };
      return;
    }
    const a = (e.target as HTMLElement).closest<HTMLElement>('[data-link]');
    if (a) {
      e.preventDefault();
      e.stopPropagation();
      hideTip();
      systemIpc.openChatLink(a.dataset.link!).catch((err) => toast.error(errorText(err)));
      return;
    }
    open = !open;
  }

  function textKey(e: KeyboardEvent) {
    if (e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
    e.preventDefault();
    open = !open;
  }

  /** The same tips as chat lines: links, emotes, badges, emoji. */
  const TIPS = '[data-tip],[data-link],[data-emoji]';

  function textOver(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest<HTMLElement>(TIPS);
    if (!a) return;
    if (a.dataset.link) {
      if (a.dataset.host) showTip(a, { label: `Opens ${a.dataset.host}` });
    } else if (a.dataset.emoji) {
      emojiName(a.dataset.emoji)
        .then((name) => name && a.matches(':hover') && showTip(a, { label: name }))
        .catch(() => {});
    } else showTip(a, { label: a.dataset.tip ?? '', hint: a.dataset.hint, image: a.dataset.tipImg });
  }

  function textOut(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest(TIPS);
    if (a && !a.contains(e.relatedTarget as Node)) hideTip();
  }
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
      <!-- c-9 Not a button: its links stay links. The link tips are extra, like in chat. -->
      <!-- svelte-ignore a11y_mouse_events_have_key_events -->
      <div
        class="text"
        role="button"
        tabindex="0"
        aria-expanded={open}
        onclick={textClick}
        onkeydown={textKey}
        onmouseover={textOver}
        onmouseout={textOut}
        onblur={hideTip}
      >
        <Body fragments={pin.message.body} {channel} />
      </div>
    </div>
    {#if onunpin}
      <button
        class="close"
        class:armed
        aria-label={armed ? 'Click again to unpin' : 'Unpin'}
        use:tooltip={{ label: 'Unpin', hint: 'Takes it down for everyone.' }}
        onclick={unpin}
      >
        {#if armed}Unpin?{:else}<Icon name="trash" size={13} />{/if}
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
          class:still
          style="--from:{span.frac};--dur:{Math.round(span.left)}ms"
          aria-hidden="true"
        ></div>
      {/key}
    {/if}
  </div>
{/if}
{#if card}
  {#key card}<EmoteCard target={card} login={channel} {account} onclose={() => (card = null)} />{/key}
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
    /* s2-5 Readable on either theme, whatever the accent. */
    background: var(--accent-text);
    opacity: 0.7;
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
    cursor: default;
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
  .close.armed {
    width: auto;
    padding: 0 6px;
    border-radius: var(--radius-sm);
    background: var(--danger-solid);
    color: var(--on-solid);
    font-size: var(--text-hint);
    font-weight: 600;
  }
  .close.armed:hover {
    color: var(--on-solid);
  }
  .text:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .text :global([data-link]) {
    cursor: pointer;
  }
</style>
