<!-- Scrolling message list (1.8). Follows new messages while at the bottom, sliding them
     in when set (V103-12); pauses on hover or while the pause key is held (C5); marks where
     you stopped reading (C5); a dated line where chat crosses midnight (C25).
     Clicks and tooltips are delegated here instead of being set on every line. -->
<script lang="ts">
  import { onDestroy, tick, untrack } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { menu, openMenu as showMenu, type MenuEntry } from '../../components/menu.svelte';
  import { hideTip } from '../../components/tooltip.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Act } from '../../ipc/bindings/Act';
  import type { LinkTarget } from '../../ipc/bindings/LinkTarget';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { chatIpc } from '../../ipc/chat';
  import { chatxIpc } from '../../ipc/chatx';
  import { pluginsIpc } from '../../ipc/plugins';
  import { formatDate, formatNumber } from '../../lib/format';
  import { reduced } from '../../lib/motion';
  import { EASE_OUT_CUBIC } from '../../lib/scrollMath';
  import { settings } from '../../lib/settings.svelte';
  import { stopScroll } from '../../lib/smoothScroll';
  import { streamer } from '../../lib/streamer.svelte';
  import { toast } from '../../lib/toast.svelte';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import { emotesIpc } from '../../ipc/emotes';
  import { modSlots, type LineMod } from '../modview/lineMod';
  import { messageItems, mods, pinItems, userItems } from '../modview/moderation.svelte';
  import { allows } from '../modview/roles';
  import { standingOf } from '../modview/standing';
  import { lineKind, pluginItems } from '../plugins/pluginMenus';
  import { plugins } from '../plugins/plugins.svelte';
  import { glideMs } from './glide';
  import { nameMenu } from './nameMenu';
  import Line from './Line.svelte';
  import DateRow from '../../components/DateRow.svelte';
  import WarnBox from './WarnBox.svelte';
  import { runEntry, toItems, userMenus } from './menus.svelte';
  import { emoteMenu, enterClicks, lineTips, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
  import { plainText } from './text';
  import { readMarks, type ChatRoom } from './room.svelte';
  import { stripes } from './stripes';
  import { whispers } from '../whispers/whispers.svelte';
  import { errorText } from '../../lib/errors';
  import { copy } from '../../lib/clipboard';
  import { scrollWithin } from '../../lib/scrollWithin';

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
          slots: modSlots(c.modButtons),
        }
      : null,
  );

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
  /** Where the list was left, to count what came in since. */
  let leftAt = $state<string | undefined>();
  const newWhilePaused = $derived(paused && pausedAt !== undefined && room.messages.at(-1)?.id !== pausedAt);

  $effect(() => {
    if (following) leftAt = undefined;
    else untrack(() => (leftAt ??= room.messages.at(-1)?.id ?? ''));
  });

  /** Messages that came in since the list stopped following (pause or scroll up). */
  const fresh = $derived.by(() => {
    const id = !following ? leftAt : pausedAt;
    if (!id) return 0;
    const ms = room.messages;
    for (let i = ms.length - 1; i >= 0; i--) if (ms[i].id === id) return ms.length - 1 - i;
    return ms.length;
  });

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
    if (!lines || !list || !at.el.isConnected || reduced()) return;
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
    const key = (d: Date) => d.getFullYear() * 10_000 + d.getMonth() * 100 + d.getDate();
    // The first line gets a date too when it isn't from today (history from yesterday).
    let last = key(new Date());
    for (const m of shown) {
      const day = key(new Date(m.ts));
      if (day !== last) starts.add(m.id);
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
    tips.drop();
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
    enterClicks(e);
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

  /** What the lines' emotes, names and links act on (lineEvents.ts). */
  function host(): LineHost {
    return {
      channel: room.login,
      account: room.account,
      senderOf: (el) => {
        const line = el.closest<HTMLElement>('[data-id]');
        return line ? room.find(line.dataset.id!)?.sender?.id : undefined;
      },
      openCard: (t) => (card = t),
      openMenu,
      onlink,
    };
  }

  // V106-31/32 Read the account's 7TV editor rights early, so the first right-click has them.
  let warmed: string | null = null;
  $effect(() => {
    const account = room.account;
    if (account && account !== warmed) {
      warmed = account;
      emotesIpc.editWarm(account).catch(() => {});
    }
  });

  /** Runs a typed command for the menus, e.g. `/hideuser x`. */
  function command(text: string): Promise<boolean> {
    return chatIpc
      .send(room.view, text)
      .then((a) => {
        if (a) onaction?.(a);
        return true;
      })
      .catch((err) => {
        toast.error(errorText(err));
        return false;
      });
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

  /** Right-click a name: the same menu as in the viewer list (nameMenu.ts). */
  function userMenu(e: MouseEvent, login: string, m: ChatMessage | undefined) {
    openMenu(
      e,
      nameMenu({
        channel: room.login,
        account: room.account,
        view: room.view,
        login,
        msg: m,
        standing: standingOf(badgesOf(login, m)),
        run: command,
        onaction,
        onmention,
        onprefill,
      }),
    );
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
              run: () => chatxIpc.translate(room.view, m.id).catch((err) => toast.error(errorText(err))),
            },
            { label: 'Their messages', icon: 'search' as const, run: () => onaction?.({ type: 'search', query: `from:${login}` }) },
          ]
        : []),
      { label: 'Copy message', icon: 'copy', run: () => void copy(plainText(m.body), 'Message') },
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
        pluginsIpc.runMenu(room.view, p.plugin, p.index, m.id).catch((err) => toast.error(errorText(err))),
      ),
    ]);
  }

  /** E4 Right-click an emote: card, copy, hide. Names and messages get their own menus. */
  function oncontextmenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const el = target.closest<HTMLElement>('[data-emote]');
    if (el) return void emoteMenu(e, el, host());
    notEmote();
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
  }

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(
      `${PARTS},[data-reply],[data-jump],[data-whisper],[data-mod],[data-pin],[data-act]`,
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
    } else if (openPart(e, el, host())) {
      return;
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
    if (target) scrollWithin(target, 'center');
    // Reduce motion: lit, then off, without the fade.
    target?.animate([{ background: 'var(--accent-wash)' }, { background: 'transparent' }], {
      duration: 1200,
      easing: reduced() ? 'steps(1, end)' : 'linear',
    });
  }

  /** S7 A plugin's piece was clicked. Links go through the link path instead. */
  function actClick(a: Act) {
    if (a.kind === 'user') chatIpc.openUsercard(room.login, a.login).catch((err) => toast.error(errorText(err)));
    else if (a.kind === 'run') command(a.text);
    else if (a.kind === 'jump') jump(a.id);
    else if (a.kind === 'copy') void copy(a.text, 'Text');
    else if (a.kind === 'insert') oninsert?.(a.text);
  }

  /** M10 The one open reason box: Warn on a line asks for the reason there. */
  let warning = $state<{ login: string; anchor: { left: number; top: number; bottom: number; right: number } } | null>(null);

  async function sendWarn(text: string) {
    const who = warning?.login ?? '';
    const ok = await command(text);
    if (ok) toast.success(`Warned ${who}.`);
    return ok;
  }

  /** V101-33 Mod mode's buttons (M10): delete the line, time out, warn or ban its sender. */
  function modClick(el: HTMLElement, action: string) {
    const id = el.closest<HTMLElement>('[data-id]')?.dataset.id;
    const login = el.closest<HTMLElement>('[data-login]')?.dataset.login;
    const secs = Number(el.dataset.secs);
    if (!id || !login) return;
    if (action === 'delete') command(`/delete ${id}`);
    else if (action === 'timeout' && secs > 0) command(`/timeout ${login} ${secs}`);
    else if (action === 'warn') {
      const r = el.getBoundingClientRect();
      // The button's tip would sit on the box's field.
      hideTip();
      warning = { login, anchor: { left: r.left, top: r.top, bottom: r.bottom, right: r.right } };
    }
    else if (action === 'ban') command(`/ban ${login}`);
  }

  // c-4 Tips for badges, links and emoji; one that loses its line goes with it.
  const tips = lineTips(() => lines);
  onDestroy(tips.drop);
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
    onmouseover={tips.onmouseover}
    onmouseout={tips.onmouseout}
    {onmousemove}
    onmouseenter={() => ((hovering = true), onmousemove())}
    {onmouseleave}
    role="log"
    data-arrows
    aria-live="off"
  >
    <div bind:this={lines}>
      {#each shown as m (m.id)}
        {#if dayStarts.has(m.id)}<DateRow label={formatDate(new Date(m.ts))} />{/if}
        <Line {m} channel={room.login} {canReply} stripe={stripe?.get(m.id) ?? false} mod={lineMod} />
        {#if m.id === lastRead && m.id !== shown.at(-1)?.id}<hr class="last-read" />{/if}
      {/each}
    </div>
  </div>
  {#if warning}
    {#key warning}<WarnBox login={warning.login} anchor={warning.anchor} send={sendWarn} onclose={() => (warning = null)} />{/key}
  {/if}
  {#if card}
    {#key card}<EmoteCard target={card} login={room.login} account={room.account} onclose={() => (card = null)} />{/key}
  {/if}
  {#if below && (!following || newWhilePaused)}
    <div class="fade" aria-hidden="true"></div>
    <button class="more" onclick={jumpDown}>
      <Icon name="arrowDown" size={13} />
      {fresh > 0 ? `${formatNumber(fresh)} new` : following ? 'Paused' : 'More messages'}
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
  .last-read {
    margin: 2px 10px;
    border: 0;
    border-top: 2px solid var(--red);
    opacity: 0.7;
  }
  /* A short fade under the pill, so it never sits on top of bare text. */
  .fade {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    height: 44px;
    background: linear-gradient(transparent, var(--page));
    pointer-events: none;
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
    box-shadow: var(--shadow-pop);
    font-size: 0.8rem;
    font-weight: 600;
  }
  .more:hover {
    background: var(--elevated-2);
  }
</style>
