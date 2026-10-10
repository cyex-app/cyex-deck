<!-- Scrolling message list (1.8). Follows new messages while at the bottom; pauses on
     hover or while the pause key is held (C5); marks where you stopped reading (C5).
     Clicks and tooltips are delegated here instead of being set on every line. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import { hideTip, retitleTip, showTip } from '../../components/tooltip.svelte';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import { chatIpc } from '../../ipc/chat';
  import { systemIpc } from '../../ipc/system';
  import { settings } from '../../lib/settings.svelte';
  import { toast } from '../../lib/toast.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { hidden } from '../emotes/hidden.svelte';
  import Line from './Line.svelte';
  import { readMarks, type ChatRoom } from './room.svelte';

  interface Props {
    room: ChatRoom;
    /** Where the last-read line sits is kept under this key while the view is closed. */
    readKey?: string;
    canReply: boolean;
    onreply: (id: string) => void;
    /** Twitch clips and VODs open in the app (1.19, 1.20, P7) instead of the browser. */
    onlink?: (target: LinkTarget) => void;
  }

  let { room, readKey, canReply, onreply, onlink }: Props = $props();

  const c = $derived(settings.v.chat);

  let list = $state<HTMLElement>();
  /** Stays true while the user wants new messages in view. Scrolling up ends it. */
  let following = $state(true);
  let hovering = $state(false);
  let keyHeld = $state(false);
  // svelte-ignore state_referenced_locally
  let lastRead = $state<string | null>((readKey && readMarks.get(readKey)) || null);
  const paused = $derived((hovering && c.pauseOnHover) || keyHeld);
  /** Last message when the pause began, to tell if anything arrived since. */
  let pausedAt = $state<string | undefined>();
  const newWhilePaused = $derived(paused && pausedAt !== undefined && room.messages.at(-1)?.id !== pausedAt);

  $effect(() => {
    if (!paused) {
      pausedAt = undefined;
      return;
    }
    untrack(() => {
      if (pausedAt === undefined) pausedAt = room.messages.at(-1)?.id ?? '';
    });
  });

  // Follow new messages unless paused or scrolled up. Also catches up after a pause.
  $effect(() => {
    void room.messages;
    if (following && !paused) void tick().then(toBottom);
  });

  // C5 Pause key.
  $effect(() => {
    const key = c.pauseKey;
    if (key === 'none') return;
    const prop = ({ ctrl: 'ctrlKey', alt: 'altKey', shift: 'shiftKey' } as const)[key];
    const sync = (e: KeyboardEvent) => (keyHeld = e[prop]);
    const release = () => (keyHeld = false);
    addEventListener('keydown', sync);
    addEventListener('keyup', sync);
    addEventListener('blur', release);
    return () => {
      removeEventListener('keydown', sync);
      removeEventListener('keyup', sync);
      removeEventListener('blur', release);
    };
  });

  // C5 Last-read line: placed when the window loses focus or the tab is left.
  $effect(() => {
    if (!c.lastReadLine) {
      lastRead = null;
      return;
    }
    const mark = () => (lastRead = room.messages.at(-1)?.id ?? null);
    addEventListener('blur', mark);
    return () => {
      removeEventListener('blur', mark);
      const last = room.messages.at(-1)?.id;
      if (readKey && last) readMarks.set(readKey, last);
    };
  });

  function toBottom() {
    if (list) list.scrollTop = list.scrollHeight;
  }

  /** Set while the user drags the scrollbar. */
  let dragging = false;

  // Only the user leaves the bottom: wheel, keys or the scrollbar. Lines loading above,
  // images sizing in and old lines dropping off also move the position; while
  // following, those just snap back down.
  function onscroll() {
    if (!list) return;
    const bottom = list.scrollTop + list.clientHeight >= list.scrollHeight - 24;
    if (bottom) following = true;
    else if (dragging) following = false;
    else if (following && !paused) toBottom();
  }

  function onwheel(e: WheelEvent) {
    if (e.deltaY < 0) following = false;
  }

  function onkeydown(e: KeyboardEvent) {
    if (['PageUp', 'ArrowUp', 'Home'].includes(e.key)) following = false;
  }

  function onpointerdown(e: PointerEvent) {
    if (!list || e.clientX < list.getBoundingClientRect().right - 14) return;
    dragging = true;
    addEventListener('pointerup', () => (dragging = false), { once: true });
  }

  function jumpDown() {
    following = true;
    hovering = false;
    keyHeld = false;
    toBottom();
  }

  /** 1.41 The emote card on screen. */
  let card = $state<CardTarget | null>(null);

  function emoteAt(el: HTMLElement): CardTarget {
    const d = JSON.parse(el.dataset.emote!) as Omit<CardTarget, 'rect'>;
    return { ...d, rect: el.getBoundingClientRect() };
  }

  /** E4 Right-click an emote: card, copy, hide. */
  function oncontextmenu(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-emote]');
    if (!el) return;
    e.preventDefault();
    const t = emoteAt(el);
    const { emote } = t;
    hideTip();
    openMenu(e, [
      { label: 'Emote card', run: () => (card = t) },
      { label: 'Copy name', run: () => void navigator.clipboard.writeText(emote.name) },
      'separator',
      {
        label: `Hide ${emote.name}`,
        run: () => hidden.hide({ source: emote.provider, id: emote.id, name: emote.name }),
      },
    ]);
  }

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(
      '[data-user],[data-link],[data-reply],[data-jump],[data-emote]',
    );
    if (!el) return;
    const d = el.dataset;
    if (d.emote) {
      hideTip();
      card = emoteAt(el);
    } else if (d.link) {
      e.preventDefault();
      if (d.target && onlink) return onlink(JSON.parse(d.target) as LinkTarget);
      systemIpc.openChatLink(d.link).catch((err) => toast.error(String(err)));
    } else if (d.user) {
      chatIpc.openUsercard(room.login, d.user).catch((err) => toast.error(String(err)));
    } else if (d.reply) {
      onreply(d.reply);
    } else if (d.jump) {
      const target = list?.querySelector<HTMLElement>(`[data-id="${CSS.escape(d.jump)}"]`);
      target?.scrollIntoView({ block: 'center' });
      target?.animate([{ background: 'var(--accent-wash)' }, { background: 'transparent' }], { duration: 1200 });
    }
  }

  function onmouseover(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-tip],[data-link]');
    if (!el) return;
    if (el.dataset.link) return showLinkTip(el);
    showTip(el, { label: el.dataset.tip ?? '', hint: el.dataset.hint, image: el.dataset.tipImg });
  }

  function onmouseout(e: MouseEvent) {
    const from = (e.target as HTMLElement).closest('[data-tip],[data-link]');
    if (from && !from.contains(e.relatedTarget as Node)) hideTip();
  }

  /** X2: the real domain right away, the final one once redirects are followed. */
  function showLinkTip(el: HTMLElement) {
    const url = el.dataset.link!;
    const host = el.dataset.host!;
    showTip(el, { label: `Opens ${host}` });
    chatIpc
      .resolveLink(url)
      .then((final) => {
        if (!final) return;
        const finalHost = new URL(final).hostname.replace(/^www\./, '');
        if (finalHost !== host) retitleTip(`Opens ${host}, ends up on ${finalHost}`);
      })
      .catch(() => {});
  }
</script>

<div class="wrap">
  <!-- Clicks and hovers are delegated from the lines' links, names and buttons, which
       are the interactive parts. -->
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions, a11y_mouse_events_have_key_events -->
  <div
    class="list"
    class:alt={c.alternateLines}
    class:sep={c.separators}
    bind:this={list}
    {onscroll}
    {onwheel}
    {onkeydown}
    {onpointerdown}
    {onclick}
    {oncontextmenu}
    {onmouseover}
    {onmouseout}
    onmouseenter={() => (hovering = true)}
    onmouseleave={() => (hovering = false)}
    role="log"
    aria-live="off"
  >
    {#each room.messages as m (m.id)}
      <Line {m} channel={room.login} {canReply} />
      {#if m.id === lastRead && m.id !== room.messages.at(-1)?.id}<hr class="last-read" />{/if}
    {/each}
  </div>
  {#if card}
    {#key card}<EmoteCard target={card} login={room.login} onclose={() => (card = null)} />{/key}
  {/if}
  {#if !following || newWhilePaused}
    <button class="more" onclick={jumpDown}>
      <Icon name="arrowDown" size={13} />
      {following ? 'Paused' : 'More messages'}
    </button>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
  }
  .list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
  }
  .alt :global(.line:nth-of-type(even)) {
    background: var(--row-alt);
  }
  .sep :global(.line) {
    border-bottom: 1px solid var(--border);
  }
  .last-read {
    margin: 2px 10px;
    border: 0;
    border-top: 2px solid var(--red);
    opacity: 0.7;
  }
  .more {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding: 0 12px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 4px 14px var(--shadow);
    font-size: 0.8rem;
    font-weight: 600;
  }
  .more:hover {
    background: var(--elevated-2);
  }
</style>
