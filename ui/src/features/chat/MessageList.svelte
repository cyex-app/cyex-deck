<!-- Scrolling message list (1.8). Follows new messages while at the bottom, sliding them
     in when set (V103-12); pauses on hover or while the pause key is held (C5); marks where
     you stopped reading (C5); a dated line where chat crosses midnight (C25).
     Clicks and tooltips are delegated here instead of being set on every line. -->
<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { menu, openMenu as showMenu, type MenuEntry } from '../../components/menu.svelte';
  import { hideTip, previewTip, retitleTip, showTip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Act } from '../../ipc/bindings/Act';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { popOut } from '../../app/panels';
  import { chatIpc } from '../../ipc/chat';
  import { chatxIpc } from '../../ipc/chatx';
  import { pluginsIpc } from '../../ipc/plugins';
  import { systemIpc } from '../../ipc/system';
  import { formatDate } from '../../lib/format';
  import { EASE_OUT_CUBIC } from '../../lib/scrollMath';
  import { settings } from '../../lib/settings.svelte';
  import { stopScroll } from '../../lib/smoothScroll';
  import { streamer } from '../../lib/streamer.svelte';
  import { toast } from '../../lib/toast.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import { emojiName } from '../emotes/emoji';
  import type { CardTarget } from '../emotes/emotes';
  import { hidden } from '../emotes/hidden.svelte';
  import { flipUnlisted, unlistedBlurred } from '../emotes/unlisted';
  import type { LineMod } from '../modview/lineMod';
  import { messageItems, mods, pinItems, userItems } from '../modview/moderation.svelte';
  import { allows } from '../modview/roles';
  import { standingOf } from '../modview/standing';
  import { lineKind, pluginItems } from '../plugins/pluginMenus';
  import { plugins } from '../plugins/plugins.svelte';
  import { glideMs } from './glide';
  import Line from './Line.svelte';
  import { runEntry, toItems, userMenus } from './menus.svelte';
  import { roomRoute } from './roomRoute';
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
    /** M10 Puts text in the message box, for a warning's reason. */
    onprefill?: (text: string) => void;
    /** S7 Puts text at the caret in the message box. */
    oninsert?: (text: string) => void;
    /** A command asked the window to do something (search, pop out). */
    onaction?: (a: UiAction) => void;
  }

  let { room, readKey, canReply, onreply, onlink, onmention, onprefill, oninsert, onaction }: Props = $props();

  const c = $derived(settings.v.chat);
  /** 1.57 Only where this account moderates: the pin button and mod mode (V101-33/34). */
  const role = $derived(mods.role(room.account, room.login));
  const lineMod = $derived<LineMod | null>(
    role
      ? {
          me: room.account,
          broadcaster: allows(role, 'broadcaster'),
          inline: mods.modMode(room.login),
          buttons: c.modButtons,
        }
      : null,
  );
  /** 1.52 Picture widths, Chatterino's sizes. */
  const THUMBS = { off: 0, small: 100, medium: 200, large: 300 } as const;

  let list = $state<HTMLElement>();
  /** The lines inside `list`; V103-12 slides move this. */
  let lines = $state<HTMLElement>();
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
  /** V101-15 The mouse rested over chat for the set time: chat moves on until it moves. */
  let still = $state(false);
  let stillTimer: ReturnType<typeof setTimeout> | undefined;
  const paused = $derived((hovering && c.pauseOnHover && !still) || keyHeld || (menuHere && menu.open));

  function onmousemove() {
    still = false;
    clearTimeout(stillTimer);
    if (c.pauseOnHover && c.pauseStillTenths) stillTimer = setTimeout(() => (still = true), c.pauseStillTenths * 100);
  }

  function onmouseleave() {
    hovering = false;
    still = false;
    clearTimeout(stillTimer);
  }
  onDestroy(() => clearTimeout(stillTimer));

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

  /** V103-12 A line and where it showed, to slide the lines in from. */
  type Mark = { el: Element; top: number };
  /** Where the newest line showed before new ones were put in. */
  let before: Mark | null = null;
  /** The slide playing, when it ends, and when lines last came (all `performance.now()`). */
  let slide: Animation | null = null;
  let slideEnds = 0;
  let lastLines = -Infinity;

  $effect.pre(() => {
    void room.messages;
    untrack(() => (before = c.smoothNewMessages && following && !paused ? mark() : null));
  });

  // Follow new messages unless paused or scrolled up. Also catches up after a pause.
  $effect(() => {
    void room.messages;
    const follow = following && !paused;
    void tick().then(() => {
      if (!follow) return measure();
      const at = before ?? (c.smoothNewMessages ? mark() : null);
      before = null;
      const top = list?.scrollTop;
      toBottom();
      // Nothing new on screen (a filtered line): a playing slide goes on.
      if (at && list?.scrollTop !== top) glide(at);
      watchNewest();
    });
  });

  // New lines get their real height a frame late (content-visibility), so a line that
  // wraps grows after the jump down without a scroll event. Snap back down then; a slide
  // that's playing takes the growth along for the rest of its time.
  const grown = new ResizeObserver(() => {
    if (!following || paused || !list) return;
    const at = slide ? mark() : null;
    const top = list.scrollTop;
    toBottom();
    if (at && list.scrollTop !== top) glide(at, true);
  });
  onDestroy(() => grown.disconnect());

  function watchNewest() {
    grown.disconnect();
    let el = lines?.lastElementChild;
    for (let i = 0; el && i < 10; i++, el = el.previousElementSibling) grown.observe(el);
  }

  function mark(): Mark | null {
    const el = lines?.lastElementChild;
    return el ? { el, top: el.getBoundingClientRect().top } : null;
  }

  /** V103-12 Already at the bottom: the lines move from where `at` showed, on the
   * compositor (a transform), so scrolling and stick-to-bottom never wait on it. Fast
   * chats skip or shorten it (glide.ts). `carry` keeps the playing slide's end time. */
  function glide(at: Mark, carry = false) {
    const left = carry && slide ? slideEnds - performance.now() : 0;
    stopGlide();
    if (!lines || !list || !at.el.isConnected) return;
    const distance = at.top - at.el.getBoundingClientRect().top;
    if (!(distance >= 1)) return;
    const now = performance.now();
    const ms = carry
      ? Math.min(left, glideMs(distance, list.clientHeight, Infinity))
      : glideMs(distance, list.clientHeight, now - lastLines);
    if (!carry) lastLines = now;
    if (ms <= 0) return;
    slide = lines.animate([{ transform: `translateY(${distance}px)` }, { transform: 'none' }], {
      duration: ms,
      easing: EASE_OUT_CUBIC,
    });
    slideEnds = now + ms;
  }

  /** The user scrolls or jumps: the lines go where they really are. */
  function stopGlide() {
    slide?.cancel();
    slide = null;
  }

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
    if (list) {
      stopScroll(list);
      list.scrollTop = list.scrollHeight;
    }
    measure();
  }

  function measure() {
    if (list) below = list.scrollTop + list.clientHeight < list.scrollHeight - 24;
  }

  /** V101-31 An empty or short chat has nothing to scroll to. */
  const scrolls = () => !!list && list.scrollHeight > list.clientHeight;

  /** Set while the user drags the scrollbar. */
  let dragging = false;
  /** Where the list was at the last scroll event, to tell up from down. */
  let lastTop = 0;

  // Only the user leaves the bottom: wheel, keys or the scrollbar. Lines loading above,
  // images sizing in and old lines dropping off also move the position; while
  // following, those just snap back down. Following starts again on the way down, or
  // when nothing is left to scroll: a smooth scroll up (V103-11) spends its first frames
  // near the bottom.
  function onscroll() {
    if (!list) return;
    measure();
    const down = list.scrollTop >= lastTop;
    lastTop = list.scrollTop;
    if (!below && (down || !scrolls())) following = true;
    else if (dragging) following = false;
    else if (following && !paused) toBottom();
  }

  function leave() {
    following = false;
    stopGlide();
  }

  function onwheel(e: WheelEvent) {
    if (e.deltaY < 0 && scrolls()) leave();
  }

  function onkeydown(e: KeyboardEvent) {
    if (['PageUp', 'ArrowUp', 'Home'].includes(e.key) && scrolls()) leave();
  }

  function onpointerdown(e: PointerEvent) {
    if (!list || e.clientX < list.getBoundingClientRect().right - 14) return;
    dragging = true;
    stopGlide();
    addEventListener('pointerup', () => (dragging = false), { once: true });
  }

  function jumpDown() {
    following = true;
    hovering = false;
    keyHeld = false;
    stopGlide();
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

  /** Warn asks for the reason in the message box (M10). */
  const warn = (login: string) => () => onprefill?.(`/warn ${login} `);

  /** M12 Their badges: the line the name was on, else their latest line here. */
  function badgesOf(login: string, m: ChatMessage | undefined) {
    if (m?.sender?.login === login) return m.sender.badges;
    for (let i = room.messages.length - 1; i >= 0; i--) {
      const s = room.messages[i].sender;
      if (s?.login === login) return s.badges;
    }
    return undefined;
  }

  /** Right-click a name: usercard, mention, search, mod actions, hide or block, your own (S1),
   * plugins' (S11). Same order as every menu (V101-30): main action, others, go somewhere,
   * copy, then red last. */
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
      ...(canMod ? ['separator' as const, ...userItems(role, login, command, warn(login), standingOf(badgesOf(login, m)))] : []),
      'separator',
      { label: 'Hide for me', icon: 'eyeOff', note: 'Only for you, not on Twitch', run: () => command(`/hideuser ${login}`) },
      ...(room.account ? [{ label: 'Block on Twitch', icon: 'ban' as const, danger: true, run: () => command(`/block ${login}`) }] : []),
      ...(own.length ? ['separator' as const, ...toItems(own, (cmd) => runEntry(room.view, cmd, ctx, onaction))] : []),
      'separator',
      ...pluginItems(plugins.menus, 'name', null, (p) =>
        pluginsIpc
          .runName(p.plugin, p.index, room.login, login, room.view, m?.id ?? null)
          .catch((err) => toast.error(String(err))),
      ),
    ]);
  }

  /** Right-click a message: reply, copy, translate (C17), your own entries (S1), plugins' (S5). */
  function messageMenu(e: MouseEvent, m: ChatMessage) {
    const own = userMenus.current.message;
    const login = m.sender?.login ?? '';
    const ctx = context(m, login);
    const role = mods.role(room.account, room.login);
    const canMod = role && m.sender && !m.notice && login !== room.login;
    // V101-34 The broadcaster's messages can be pinned too.
    const canPin = role && m.sender && !m.notice && !canMod;
    openMenu(e, [
      ...(canReply && m.sender && !m.notice ? [{ label: 'Reply', icon: 'reply' as const, run: () => onreply(m.id) }] : []),
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
      { label: 'Copy message', icon: 'copy', run: () => void navigator.clipboard.writeText(plainText(m.body)) },
      ...(canMod
        ? [
            'separator' as const,
            ...messageItems(m.id, command),
            'separator' as const,
            ...userItems(role, login, command, warn(login), standingOf(m.sender?.badges)),
          ]
        : []),
      ...(canPin ? ['separator' as const, { label: 'Pin message', icon: 'pin' as const, items: pinItems(m.id, command) }] : []),
      ...(own.length ? ['separator' as const, ...toItems(own, (cmd) => runEntry(room.view, cmd, ctx, onaction))] : []),
      // S5, S9 Entries plugins added, for this kind of line.
      'separator',
      ...pluginItems(plugins.menus, 'message', lineKind(m), (p) =>
        pluginsIpc.runMenu(room.view, p.plugin, p.index, m.id).catch((err) => toast.error(String(err))),
      ),
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
      { label: 'Emote card', icon: 'smile', run: () => (card = t) },
      { label: 'Copy name', icon: 'copy', run: () => void navigator.clipboard.writeText(emote.name) },
      'separator',
      {
        label: `Hide ${emote.name}`,
        icon: 'eyeOff',
        run: () => hidden.hide({ source: emote.provider, id: emote.id, name: emote.name }),
      },
      // V102-01 One switch for every unlisted 7TV emote, in every chat.
      ...(el.dataset.unlisted !== undefined
        ? ['separator' as const, { label: 'Blur unlisted emotes', icon: 'eye' as const, checked: unlistedBlurred(), run: flipUnlisted }]
        : []),
    ]);
  }

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(
      '[data-user],[data-channel],[data-link],[data-reply],[data-jump],[data-emote],[data-whisper],[data-mod],[data-pin],[data-act]',
    );
    if (!el) return;
    const d = el.dataset;
    if (d.mod) {
      modClick(el, d.mod);
    } else if (d.act) {
      actClick(JSON.parse(d.act) as Act);
    } else if (d.pin) {
      hideTip();
      menuHere = true;
      showMenu(el, pinItems(d.pin, command), 'end');
    } else if (d.emote) {
      hideTip();
      card = emoteAt(el);
    } else if (d.link) {
      e.preventDefault();
      if (d.target && onlink) return onlink(JSON.parse(d.target) as LinkTarget);
      systemIpc.openChatLink(d.link).catch((err) => toast.error(String(err)));
    } else if (d.channel) {
      // V102-05 As this chat's account, or anonymously like it.
      hideTip();
      void popOut(roomRoute(d.channel, room.account), `${d.channel} chat`);
    } else if (d.user) {
      chatIpc.openUsercard(room.login, d.user).catch((err) => toast.error(String(err)));
    } else if (d.whisper) {
      whispers.show(d.whisper);
    } else if (d.reply) {
      onreply(d.reply);
    } else if (d.jump) {
      jump(d.jump);
    }
  }

  function jump(id: string) {
    stopGlide();
    if (list) stopScroll(list);
    const target = list?.querySelector<HTMLElement>(`[data-id="${CSS.escape(id)}"]`);
    target?.scrollIntoView({ block: 'center' });
    target?.animate([{ background: 'var(--accent-wash)' }, { background: 'transparent' }], { duration: 1200 });
  }

  /** S7 A plugin's piece was clicked. Links go through the link path instead. */
  function actClick(a: Act) {
    if (a.kind === 'user') chatIpc.openUsercard(room.login, a.login).catch((err) => toast.error(String(err)));
    else if (a.kind === 'run') command(a.text);
    else if (a.kind === 'jump') jump(a.id);
    else if (a.kind === 'copy') void navigator.clipboard.writeText(a.text).then(() => toast.info('Copied.'));
    else if (a.kind === 'insert') oninsert?.(a.text);
  }

  /** V101-33 Mod mode's buttons (M10): delete the line, time out, warn or ban its sender. */
  function modClick(el: HTMLElement, action: string) {
    const id = el.closest<HTMLElement>('[data-id]')?.dataset.id;
    const login = el.closest<HTMLElement>('[data-login]')?.dataset.login;
    const secs = Number(el.dataset.secs);
    if (!id || !login) return;
    if (action === 'delete') command(`/delete ${id}`);
    else if (action === 'timeout' && secs > 0) command(`/timeout ${login} ${secs}`);
    else if (action === 'warn') onprefill?.(`/warn ${login} `);
    else if (action === 'ban') command(`/ban ${login}`);
  }

  function onmouseover(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-tip],[data-link],[data-emoji]');
    if (!el) return;
    if (el.dataset.link) return showLinkTip(el);
    if (el.dataset.emoji) return showEmojiTip(el);
    showTip(el, { label: el.dataset.tip ?? '', hint: el.dataset.hint, image: el.dataset.tipImg });
  }

  function onmouseout(e: MouseEvent) {
    const from = (e.target as HTMLElement).closest('[data-tip],[data-link],[data-emoji]');
    if (from && !from.contains(e.relatedTarget as Node)) hideTip();
  }

  /** V102-07 An emoji's name, once the list has loaded (the first time only). */
  function showEmojiTip(el: HTMLElement) {
    emojiName(el.dataset.emoji!)
      .then((name) => name && el.matches(':hover') && showTip(el, { label: name }))
      .catch(() => {});
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
    {onmousemove}
    onmouseenter={() => ((hovering = true), onmousemove())}
    {onmouseleave}
    role="log"
    aria-live="off"
  >
    <div bind:this={lines}>
      {#each shown as m (m.id)}
        {#if dayStarts.has(m.id)}<p class="day" role="separator">{formatDate(new Date(m.ts))}</p>{/if}
        <Line {m} channel={room.login} {canReply} stripe={stripe?.get(m.id) ?? false} mod={lineMod} />
        {#if m.id === lastRead && m.id !== shown.at(-1)?.id}<hr class="last-read" />{/if}
      {/each}
    </div>
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
  /* V102-17 min-width 0: as a flex item of the chat's body it would otherwise be as wide as
     the widest line that can't wrap (a reply's one-line "Replying to …" header), and every
     line in a narrow chat got that width and was cut off on the right. */
  .wrap {
    position: relative;
    flex: 1;
    min-width: 0;
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
