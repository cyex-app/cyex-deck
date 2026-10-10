<!-- Scrolling message list (1.8). Follows new messages while at the bottom; pauses on
     hover or while the pause key is held (C5); marks where you stopped reading (C5); a
     dated line where chat crosses midnight (C25).
     Clicks and tooltips are delegated here instead of being set on every line. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { menu, openMenu as showMenu, type MenuEntry } from '../../components/menu.svelte';
  import { hideTip, previewTip, retitleTip, showTip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { chatIpc } from '../../ipc/chat';
  import { chatxIpc } from '../../ipc/chatx';
  import { pluginsIpc } from '../../ipc/plugins';
  import { systemIpc } from '../../ipc/system';
  import { formatDate } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import { streamer } from '../../lib/streamer.svelte';
  import { toast } from '../../lib/toast.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { hidden } from '../emotes/hidden.svelte';
  import { messageItems, mods, userItems } from '../modview/moderation.svelte';
  import { plugins } from '../plugins/plugins.svelte';
  import Line from './Line.svelte';
  import { runEntry, toItems, userMenus } from './menus.svelte';
  import { plainText } from './text';
  import { readMarks, type ChatRoom } from './room.svelte';
  import { stripes } from './stripes';
  import { whispers } from '../whispers/whispers.svelte';
  import { accounts } from '../../lib/accounts.svelte';

  interface Props {
    room: ChatRoom;
    /** Where the last-read line sits is kept under this key while the view is closed. */
    readKey?: string;
    canReply: boolean;
    onreply: (id: string) => void;
    /** Twitch clips and VODs open in the app (1.19, 1.20, P7) instead of the browser. */
    onlink?: (target: LinkTarget) => void;
    /** Puts `@login` in the message box. */
    onmention?: (login: string) => void;
    /** A command asked the window to do something (search, pop out). */
    onaction?: (a: UiAction) => void;
  }

  let { room, readKey, canReply, onreply, onlink, onmention, onaction }: Props = $props();

  const c = $derived(settings.v.chat);
  /** 1.52 Picture widths, Chatterino's sizes. */
  const THUMBS = { off: 0, small: 100, medium: 200, large: 300 } as const;

  let list = $state<HTMLElement>();
  /** Stays true while the user wants new messages in view. Scrolling up ends it. */
  let following = $state(true);
  /** V101-31 Lines out of view below. Without any there's no "More messages". */
  let below = $state(false);
  let hovering = $state(false);
  let keyHeld = $state(false);
  // svelte-ignore state_referenced_locally
  let lastRead = $state<string | null>((readKey && readMarks.get(readKey)) || null);
  /** A menu opened from this list keeps it still until it closes. */
  let menuHere = $state(false);
  const paused = $derived((hovering && c.pauseOnHover) || keyHeld || (menuHere && menu.open));

  function openMenu(e: MouseEvent, items: MenuEntry[]) {
    menuHere = true;
    showMenu(e, items);
  }

  $effect(() => {
    if (!menu.open) menuHere = false;
  });
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
    const follow = following && !paused;
    void tick().then(() => (follow ? toBottom() : measure()));
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

  /** F6 Near-duplicates go when the setting says hide. */
  const hideSimilar = $derived(settings.v.highlights.similar.hide);
  /** U4 Streamer mode keeps mod actions, deleted lines and whispers off the stream. */
  const hideMod = $derived(streamer.hides('hideModActions'));
  const hideWhispers = $derived(streamer.hides('hideWhispers'));
  const shown = $derived(
    hideSimilar || hideMod || hideWhispers
      ? room.messages.filter(
          (m) =>
            !(hideSimilar && m.flags.includes('similar')) &&
            !(hideMod && (m.notice?.kind === 'moderation' || m.flags.includes('deleted'))) &&
            !(hideWhispers && m.flags.includes('whisper')),
        )
      : room.messages,
  );

  $effect(() => userMenus.track());

  /** C25 Messages that start a new day, by id. */
  const dayStarts = $derived.by(() => {
    const starts = new Set<string>();
    let last = -1;
    for (const m of shown) {
      const d = new Date(m.ts);
      const day = d.getFullYear() * 10_000 + d.getMonth() * 100 + d.getDate();
      if (last !== -1 && day !== last) starts.add(m.id);
      last = day;
    }
    return starts;
  });

  /** V101-01 Stripe per line, kept while it's shown. */
  let striped = new Map<string, boolean>();
  const stripe = $derived.by(() => {
    if (!c.alternateLines) return null;
    striped = stripes(shown.map((m) => m.id), striped);
    return striped;
  });

  function toBottom() {
    if (list) list.scrollTop = list.scrollHeight;
    measure();
  }

  function measure() {
    if (list) below = list.scrollTop + list.clientHeight < list.scrollHeight - 24;
  }

  /** V101-31 An empty or short chat has nothing to scroll to. */
  const scrolls = () => !!list && list.scrollHeight > list.clientHeight;

  /** Set while the user drags the scrollbar. */
  let dragging = false;

  // Only the user leaves the bottom: wheel, keys or the scrollbar. Lines loading above,
  // images sizing in and old lines dropping off also move the position; while
  // following, those just snap back down.
  function onscroll() {
    if (!list) return;
    measure();
    if (!below) following = true;
    else if (dragging) following = false;
    else if (following && !paused) toBottom();
  }

  function onwheel(e: WheelEvent) {
    if (e.deltaY < 0 && scrolls()) following = false;
  }

  function onkeydown(e: KeyboardEvent) {
    if (['PageUp', 'ArrowUp', 'Home'].includes(e.key) && scrolls()) following = false;
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

  /** Runs a typed command for the menus, e.g. `/hideuser x`. */
  function command(text: string) {
    chatIpc
      .send(room.view, text)
      .then((a) => a && onaction?.(a))
      .catch((err) => toast.error(String(err)));
  }

  function context(m: ChatMessage | undefined, login: string) {
    return {
      login,
      userId: m?.sender?.login === login ? m.sender.id : null,
      display: m?.sender?.login === login ? m.sender.display : null,
      msgId: m?.id ?? null,
      msg: m ? plainText(m.body) : null,
      msgTime: m?.ts ?? null,
    };
  }

  /** Right-click a name: usercard, mention, search, mod actions, hide or block, your own (S1). */
  function userMenu(e: MouseEvent, login: string, m: ChatMessage | undefined) {
    const own = userMenus.current.user;
    const ctx = context(m, login);
    // 1.57 Mod actions only when this account moderates here, and not on the broadcaster.
    const role = mods.role(room.account, room.login);
    const canMod = role && login !== room.login;
    openMenu(e, [
      { label: 'Usercard', icon: 'user', run: () => chatIpc.openUsercard(room.login, login).catch(() => {}) },
      { label: 'Mention', icon: 'reply', run: () => onmention?.(login) },
      ...(accounts.can('whispers') ? [{ label: 'Whisper', icon: 'whisper' as const, run: () => whispers.show(login) }] : []),
      { label: 'Their messages', icon: 'search', run: () => onaction?.({ type: 'search', query: `from:${login}` }) },
      { label: 'Copy name', icon: 'copy', run: () => void navigator.clipboard.writeText(login) },
      ...(canMod ? ['separator' as const, ...userItems(role, login, command)] : []),
      'separator',
      { label: 'Hide for me', icon: 'eyeOff', hint: 'Only for you, not on Twitch', run: () => command(`/hideuser ${login}`) },
      ...(room.account ? [{ label: 'Block on Twitch', icon: 'ban' as const, danger: true, run: () => command(`/block ${login}`) }] : []),
      ...(own.length ? ['separator' as const, ...toItems(own, (cmd) => runEntry(room.view, cmd, ctx, onaction))] : []),
    ]);
  }

  /** Right-click a message: reply, copy, translate (C17), your own entries (S1), plugins' (S5). */
  function messageMenu(e: MouseEvent, m: ChatMessage) {
    const own = userMenus.current.message;
    const login = m.sender?.login ?? '';
    const ctx = context(m, login);
    const role = mods.role(room.account, room.login);
    const canMod = role && m.sender && !m.notice && login !== room.login;
    openMenu(e, [
      ...(canReply && m.sender && !m.notice ? [{ label: 'Reply', icon: 'reply' as const, run: () => onreply(m.id) }] : []),
      { label: 'Copy message', icon: 'copy', run: () => void navigator.clipboard.writeText(plainText(m.body)) },
      ...(m.sender && !m.notice
        ? [
            {
              label: m.translation ? 'Translate again' : 'Translate',
              icon: 'languages' as const,
              run: () => chatxIpc.translate(room.view, m.id).catch((err) => toast.error(String(err))),
            },
            { label: 'Their messages', icon: 'search' as const, run: () => onaction?.({ type: 'search', query: `from:${login}` }) },
          ]
        : []),
      ...(canMod ? ['separator' as const, ...messageItems(m.id, command), ...userItems(role, login, command)] : []),
      ...(own.length ? ['separator' as const, ...toItems(own, (cmd) => runEntry(room.view, cmd, ctx, onaction))] : []),
      // S5 Entries plugins added.
      ...(plugins.menus.length
        ? [
            'separator' as const,
            ...plugins.menus.map((p) => ({
              label: p.label,
              icon: 'puzzle' as const,
              run: () => pluginsIpc.runMenu(room.view, p.plugin, p.index, m.id).catch((err) => toast.error(String(err))),
            })),
          ]
        : []),
    ]);
  }

  /** E4 Right-click an emote: card, copy, hide. Names and messages get their own menus. */
  function oncontextmenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const el = target.closest<HTMLElement>('[data-emote]');
    if (!el) {
      if (target.closest('a, input, textarea')) return;
      const line = target.closest<HTMLElement>('[data-id]');
      const m = line ? room.find(line.dataset.id!) : undefined;
      const name = target.closest<HTMLElement>('[data-user]');
      if (name) {
        e.preventDefault();
        hideTip();
        userMenu(e, name.dataset.user!, m);
      } else if (m && window.getSelection()?.isCollapsed !== false) {
        e.preventDefault();
        hideTip();
        messageMenu(e, m);
      }
      return;
    }
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
      '[data-user],[data-link],[data-reply],[data-jump],[data-emote],[data-whisper]',
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
    } else if (d.whisper) {
      whispers.show(d.whisper);
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

  /** X2: the real domain right away, the final one once redirects are followed; 1.52 a
   * preview with it when previews are on. */
  function showLinkTip(el: HTMLElement) {
    const url = el.dataset.link!;
    const host = el.dataset.host!;
    showTip(el, { label: `Opens ${host}` });
    chatIpc
      .resolveLink(url)
      .then((info) => {
        if (!info) return;
        if (info.link) {
          const finalHost = new URL(info.link).hostname.replace(/^www\./, '');
          if (finalHost !== host) retitleTip(`Opens ${host}, ends up on ${finalHost}`);
        }
        if (c.linkPreviews && !streamer.hides('hideLinkPreviews') && (info.lines.length || info.thumbnail)) {
          previewTip(el, { lines: info.lines, thumbnail: info.thumbnail ?? '', thumbWidth: THUMBS[c.linkThumbnails] });
        }
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
    {#each shown as m (m.id)}
      {#if dayStarts.has(m.id)}<p class="day" role="separator">{formatDate(new Date(m.ts))}</p>{/if}
      <Line {m} channel={room.login} {canReply} stripe={stripe?.get(m.id) ?? false} />
      {#if m.id === lastRead && m.id !== shown.at(-1)?.id}<hr class="last-read" />{/if}
    {/each}
  </div>
  {#if card}
    {#key card}<EmoteCard target={card} login={room.login} onclose={() => (card = null)} />{/key}
  {/if}
  {#if below && (!following || newWhilePaused)}
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
  /* U8 Wallpaper behind chat only (design/theme.ts sets the values). */
  :global(:root[data-wall='chat']) .wrap {
    isolation: isolate;
    overflow: hidden;
  }
  :global(:root[data-wall='chat']) .wrap::before {
    content: '';
    position: absolute;
    inset: calc(var(--wall-blur) * -2);
    z-index: -1;
    pointer-events: none;
    background: var(--wall-image) center / var(--wall-size) var(--wall-repeat);
    opacity: var(--wall-opacity);
    filter: blur(var(--wall-blur));
  }
  .list {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px 0;
  }
  .sep :global(.line) {
    border-bottom: 1px solid var(--border);
  }
  .day {
    display: flex;
    align-items: center;
    gap: 8px;
    margin: 4px 10px;
    color: var(--dim);
    font: 600 0.72rem var(--font-mono);
  }
  .day::before,
  .day::after {
    content: '';
    flex: 1;
    border-top: 1px solid var(--border);
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
