<!-- 1.9 The channel's pinned message, above the chat. Can be folded to one line, hidden
     into the events button while it's pinned (V103-04) or closed until the pin changes;
     mods can take it down (V101-34) or delete the message (V107), from its ⋮ menu (V107-P4). It always starts open, with who sent and who pinned
     it, both names open their usercard (V107-P5); folding is for this pin only (V106-19). A pin that ends shows a timer bar. Its text is semibold and a step larger than chat
     (Settings → General, V105-03), so it reads at a glance. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { openMenuAt, type MenuEntry } from '../../components/menu.svelte';
  import type { Pinned } from '../../ipc/bindings/Pinned';
  import { chatIpc } from '../../ipc/chat';
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
  import { nowSecond } from '../../lib/clock.svelte';
  import { pinLeft, pinSpan } from './pinTimer';

  interface Props {
    pin: Pinned;
    channel: string;
    /** V106-31/32 The account reading the chat, for the emote card's 7TV edits. */
    account?: string | null;
    /** Only for mods (1.57). */
    onunpin?: () => void;
    /** V107 Deletes the pinned message; only for mods. */
    ondelete?: () => void;
    /** V107 Why Delete is greyed out (Twitch's rules), or null. */
    deleteWhy?: string | null;
  }

  let { pin, channel, account = null, onunpin, ondelete, deleteWhy = null }: Props = $props();

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
  /** V107 The time left, beside who pinned it; ticks on the shared 1 s clock. */
  const left = $derived.by(() => {
    const end = pin.endsAt ? Date.parse(pin.endsAt) : NaN;
    return Number.isNaN(end) ? null : pinLeft(end - nowSecond());
  });

  /** V107-P4 Mods: Unpin, Delete; everyone: Hide for yourself (paid pins have no mod items).
   * s3-1 Unpin and Delete act for everyone, so they ask once more over their own item, with
   * the menu still open, for 3 s (V107-P5). */
  function menuItems(): MenuEntry[] {
    return [
      ...(onunpin
        ? [{ label: 'Unpin', icon: 'pinOff', note: 'Takes it down for everyone.', confirm: 'Click to unpin', run: () => onunpin?.() } as const]
        : []),
      ...(ondelete
        ? [
            {
              label: 'Delete message',
              icon: 'trash',
              danger: true,
              disabled: !!deleteWhy,
              note: deleteWhy ?? 'Removes it from chat and unpins it.',
              confirm: 'Click to delete',
              run: () => ondelete?.(),
            } as const,
          ]
        : []),
      'separator',
      {
        label: 'Hide for yourself',
        icon: 'eyeOff',
        note: 'Only for you, until the next pin.',
        run: () => eventStrip.close(channel, key),
      },
    ];
  }

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
  /** V107-P5 Who pinned it, for their usercard; older reads had only the display name. */
  const byLogin = $derived(pin.byLogin ?? (pin.by && /^w+$/.test(pin.by) ? pin.by.toLowerCase() : null));

  /** V107-P5 A name opens its usercard, like a name in chat. */
  function usercard(login: string) {
    hideTip();
    chatIpc.openUsercard(channel, login).catch((err) => toast.error(errorText(err)));
  }
  function cardKey(e: KeyboardEvent, login: string) {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    usercard(login);
  }
</script>

{#if shows(eventStrip.of(channel), key)}
  <div class="pin" class:paid={pin.paid} class:open class:short bind:this={el}>
    <Icon name="pin" size={14} />
    <div class="main">
      {#if (sender && name) || pin.by || left}
        <div class="head">
          <span class="who">
            {#if sender && name}
              <span
                class="name"
                role="link"
                tabindex="0"
                aria-label="{name.name}, open usercard"
                style="--cd:{sender.color.dark};--cl:{sender.color.light}"
                onclick={() => usercard(sender.login)}
                onkeydown={(e) => cardKey(e, sender.login)}>{name.name}</span
              >
            {/if}
            {#if pin.by}<span class="by"
                >{sender && name ? '· ' : ''}Pinned by {#if byLogin}<span
                    class="pinner"
                    role="link"
                    tabindex="0"
                    aria-label="{pin.by}, open usercard"
                    onclick={() => usercard(byLogin)}
                    onkeydown={(e) => cardKey(e, byLogin)}>{pin.by}</span
                  >{:else}{pin.by}{/if}</span
              >{/if}
          </span>
          {#if left}<span class="left" aria-label="{left} left">{left}</span>{/if}
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
    <!-- V107-P4 Two controls: ⋮ (Unpin, Delete, Hide for yourself), then the chevron. -->
    <button
      class="close"
      aria-label="Pinned message options"
      aria-haspopup="menu"
      use:tooltip={{ label: 'More' }}
      onclick={(e) => {
        hideTip();
        openMenuAt(e, menuItems());
      }}
    >
      <Icon name="moreV" size={13} />
    </button>
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
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .who {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* V107 The countdown: small, steady digits, quiet like the rest of the head. */
  .left {
    flex: none;
    color: var(--dim);
    font-size: var(--text-hint);
    font-variant-numeric: tabular-nums;
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
  /* V107-P5 Both names open their usercard, like names in chat. */
  .name,
  .pinner {
    cursor: pointer;
    border-radius: var(--radius-sm);
  }
  .name:hover,
  .pinner:hover {
    text-decoration: underline;
  }
  .pinner:hover {
    color: var(--muted);
  }
  .name:focus-visible,
  .pinner:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 1px;
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
  .text:focus-visible {
    outline: 2px solid var(--focus-ring);
    outline-offset: 2px;
  }
  .text :global([data-link]) {
    cursor: pointer;
  }
</style>
