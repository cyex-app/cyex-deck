<!-- Message box (1.8, C6, C21): `@`, `:` and `!` suggestions (emotes, emoji E1, bot
     commands C13), Tab completion, the emote menu (1.40), length counter, slow mode and
     timeout countdown, replies. Enter sends; Ctrl+Enter sends and keeps the text for
     sending again. Up recalls what you sent. Pasted or dropped images go to your upload host (V105-11). -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { rankNames } from '../emotes/ranking';
  import { insert, wordAt, type Word } from './completion';
  import { formatNumber } from '../../lib/format';
  import { commandAt, commandHint, commandMatches } from './commandTab';
  import { argAt, argMatches } from './commandArgs';
  import { pluginsIpc } from '../../ipc/plugins';
  import { mods, PIN_LENGTHS } from '../modview/moderation.svelte';
  import { plugins } from '../plugins/plugins.svelte';
  import BitsButton from './BitsButton.svelte';
  import { CheerBox } from './bits.svelte';
  import { catalogs } from './catalog.svelte';
  import { cheerTotal, setCheer } from './cheer';
  import CheerConfirm from './CheerConfirm.svelte';
  import { chatxIpc } from '../../ipc/chatx';
  import EmoteButton from './EmoteButton.svelte';
  import IdentityButton from './IdentityButton.svelte';
  import RewardsMenu from './RewardsMenu.svelte';
  import type { ChatRoom } from './room.svelte';
  import { MAX_CHARS, sendLength } from './sendLength';
  import { Sources } from './sources.svelte';
  import SuggestList from './SuggestList.svelte';
  import UploadConfirm from './UploadConfirm.svelte';
  import { useUploads } from './upload.svelte';
  import type { Item } from './suggestions';
  import { pinText } from './pinSend';
  import { plainText } from './text';
  import { errorText } from '../../lib/errors';

  interface Props {
    room: ChatRoom;
    replyTo: ChatMessage | null;
    onsignin: () => void;
    /** What a command asked the window to do (S1). */
    onaction?: (a: UiAction) => void;
  }

  let { room, replyTo = $bindable(), onsignin, onaction }: Props = $props();

  /** The counter shows from here on. */
  const SHOW_COUNT = 400;
  const HISTORY = 20;

  let box = $state<HTMLTextAreaElement>();
  /** The message box with its buttons: suggestions line up with it (V102-03). */
  let row = $state<HTMLElement>();
  let text = $state('');
  let error = $state('');
  let sent: string[] = [];
  let recall = -1;
  let now = $state(Date.now());

  interface Suggest {
    word: Word;
    items: Item[];
    index: number;
    /** V1051-05 A `/combo`, `/paint` or `/badge` argument: Enter sends, Tab or a click fills. */
    arg?: boolean;
  }
  let suggest = $state<Suggest | null>(null);
  /** Tab cycling through matches for one word. */
  let cycle: { word: Word; items: string[]; index: number } | null = null;
  /** S5 Plugins are being asked for matches. */
  let asking = false;
  /** V105-11 Images pasted or dropped, and the first-upload question. */
  const up = useUploads({
    login: () => room.login,
    row: () => row,
    insert: (link) => void insertAtCaret(link),
    fail: (m) => (error = m),
  });
  /** V107-Bn A pasted cheer word asks first: a copied chat line never spends on Enter. */
  let pastedCheer = false;
  const isCheermote = (p: string) => !!catalogs.cheermote(room.login, p);
  function onpaste(e: ClipboardEvent) {
    if (cheerTotal(e.clipboardData?.getData('text') ?? '', isCheermote) > 0) pastedCheer = true;
    void up.onpaste(e);
  }
  $effect(() => {
    if (!text) pastedCheer = false;
  });

  const canChat = $derived(room.account !== null && accounts.can('sendChat', room.account));
  // V107-B Cheering: same gate as power-ups.
  const canCheer = $derived(room.account !== null && accounts.can('powerUps', room.account));
  // svelte-ignore state_referenced_locally
  const cheers = new CheerBox(() => room.view);
  /** A cheer waiting for "Spend N Bits?". `again`: the same one just went out. */
  interface CheerAsk {
    raw: string;
    value: string;
    bits: number;
    keep: boolean;
    again: boolean;
  }
  let askCheer = $state<CheerAsk | null>(null);
  // V107-N12a A /me line counts against 491, like the send.
  const length = $derived(sendLength(text));
  const chars = $derived(length.chars);
  const MAX = $derived(length.max);
  const waitMs = $derived(room.state.waitUntil ? room.state.waitUntil - now : 0);
  // The room never changes for one box.
  // svelte-ignore state_referenced_locally
  const sources = new Sources(room);

  // Ticks the countdown only while there's one to show.
  $effect(() => {
    if (!room.state.waitUntil || room.state.waitUntil <= Date.now()) return;
    const t = setInterval(() => {
      now = Date.now();
      if (now >= (room.state.waitUntil ?? 0)) clearInterval(t);
    }, 250);
    return () => clearInterval(t);
  });

  $effect(() => {
    if (replyTo) void tick().then(() => box?.focus());
  });

  // V1051-05 An emptied box drops the error it had.
  $effect(() => {
    if (!text) error = '';
  });

  // V107-N2d An error shows 4 s; a newer one gets its own 4 s (an older timer can't clear it).
  let errorTimer: ReturnType<typeof setTimeout> | undefined;
  function flash(m: string) {
    error = m;
    clearTimeout(errorTimer);
    errorTimer = setTimeout(() => (error = ''), 4000);
  }

  /** Newest argument lookup; an older answer is dropped. */
  let argSeq = 0;

  function updateSuggest() {
    // V107-N1b Typing, pasting or a click ends a Tab cycle: its word range is out of date.
    cycle = null;
    const caret = box?.selectionStart ?? text.length;
    const seq = ++argSeq;
    const arg = argAt(text, caret);
    if (arg) return void suggestArg(arg, caret, seq);
    const word = wordAt(text, caret);
    const command = commandAt(text, word);
    if (command !== null) return void suggestCommand(word, command, seq);
    const items = sources.items(word);
    suggest = items.length ? { word, items, index: 0 } : null;
  }

  /** The commands this chat can run, asked once and kept. */
  let commandNames: Promise<string[]> | null = null;

  /** A4 The first word after `/`: the commands that match. Enter still sends what's typed; Tab or
   * a click fills the name in. */
  async function suggestCommand(word: Word, typed: string, seq: number) {
    const view = room.view;
    if (!view) return void (suggest = null);
    commandNames ??= chatIpc.commands(view).catch(() => []);
    const names = await commandNames;
    if (seq !== argSeq) return;
    const found = typed ? commandMatches(typed, names, 12) : names.slice(0, 12).map((n) => `/${n}`);
    // V107 A command typed in full stays in the list, first, with its hint.
    const items = found.map((v): Item => ({ value: v, label: v, hint: commandHint(v.slice(1)) }));
    suggest = items.length ? { word, items, index: 0, arg: true } : null;
  }

  /** V1051-05 The same list as for emotes, for what `/combo `, `/paint ` and `/badge ` take. */
  async function suggestArg(arg: NonNullable<ReturnType<typeof argAt>>, caret: number, seq: number) {
    const view = room.view;
    const before = text;
    const names = view ? await chatIpc.commandArgs(view, arg.command, arg.sub).catch(() => []) : [];
    if (seq !== argSeq || text !== before) return;
    const word: Word = { start: arg.start, end: caret, text: text.slice(arg.start, caret) };
    const items = argMatches(arg, names, 12)
      // Typed in full already: nothing left to offer.
      .filter((v) => v !== word.text)
      .map((v): Item => ({ value: v, label: v.replace(/^["“]|["”]$/g, '') }));
    suggest = items.length ? { word, items, index: 0, arg: true } : null;
  }

  /** 1.40 Puts an emote or emoji at the caret, with spaces around it. */
  async function insertAtCaret(value: string) {
    const start = box?.selectionStart ?? text.length;
    const end = box?.selectionEnd ?? start;
    const before = text.slice(0, start);
    const after = text.slice(end);
    const lead = before && !before.endsWith(' ') ? ' ' : '';
    text = `${before}${lead}${value} ${after.replace(/^ /, '')}`;
    await tick();
    const caret = before.length + lead.length + value.length + 1;
    box?.setSelectionRange(caret, caret);
  }

  function focused() {
    sources.load();
  }

  /** Back to the box, e.g. after a pop-up closes. */
  export function focus() {
    box?.focus();
  }

  /** Puts `@login` in the box, from a name's menu. */
  export function mention(login: string) {
    box?.focus();
    void insertAtCaret(`@${login}`);
  }

  /** S7 Puts text at the caret, from a plugin's piece. */
  export function insertText(value: string) {
    box?.focus();
    void insertAtCaret(value);
  }

  /** M10 Replaces the box's text, caret at the end: `/warn login ` waits for the reason. */
  export async function prefill(value: string) {
    text = value;
    box?.focus();
    await tick();
    box?.setSelectionRange(value.length, value.length);
  }

  async function put(word: Word, value: string) {
    const next = insert(text, word, value);
    text = next.text;
    await tick();
    box?.setSelectionRange(next.caret, next.caret);
  }

  function accept(i = suggest?.index ?? 0) {
    if (!suggest) return;
    void put(suggest.word, suggest.items[i].value);
    suggest = null;
  }

  /** C6 Tab: complete the word, Tab again for the next match. Plugins' matches (S5) go first.
   * V1041-01 The first word after `/` completes to the commands this chat can use. */
  async function tabComplete(back: boolean) {
    if (cycle) {
      const n = cycle.items.length;
      cycle.index = (cycle.index + (back ? n - 1 : 1)) % n;
      const value = cycle.items[cycle.index];
      const word = cycle.word;
      void put(word, value);
      cycle.word = { start: word.start, end: word.start + value.length, text: value };
      return;
    }
    const before = text;
    const caret = box?.selectionStart ?? text.length;
    const word = wordAt(text, caret);
    const bare = word.text.replace(/^@/, '');
    const command = commandAt(text, word);
    const arg = argAt(text, caret);
    // V1051-04 After /combo, /paint and /badge Tab completes the argument, not the word.
    const target: Word = arg ? { start: arg.start, end: caret, text: text.slice(arg.start, caret) } : word;
    if (asking) return;
    const view = room.view;
    const own = async (): Promise<string[]> => {
      if (command !== null) {
        if (!command || !view) return [];
        return commandMatches(command, await chatIpc.commands(view).catch(() => []));
      }
      if (!bare) return [];
      const people = sources.chatters().map((n) => (word.text.startsWith('@') ? `@${n}` : n));
      // V107-R7 The emote menu's order: exact, match, most used, then menu order.
      return rankNames(bare, [...sources.emotes().map((e) => e.name), ...people], 30, sources.used());
    };
    const all = async () => {
      if (arg) {
        const names = view ? await chatIpc.commandArgs(view, arg.command, arg.sub).catch(() => []) : [];
        return argMatches(arg, names);
      }
      const fromPlugins =
        plugins.completes && view
          ? await pluginsIpc.complete(view, word.text, text, caret, word.start === 0).catch(() => null)
          : null;
      return [...(fromPlugins?.values ?? []), ...(fromPlugins?.hideOthers ? [] : await own())];
    };
    asking = true;
    const found = await all().finally(() => (asking = false));
    // The box changed while plugins or Rust thought about it.
    if (text !== before) return;
    const items = [...new Set(found)];
    if (!items.length) return;
    void put(target, items[0]);
    cycle = { word: { start: target.start, end: target.start + items[0].length, text: items[0] }, items, index: 0 };
  }

  /** A second Enter while one is on its way doesn't send twice. */
  let sending = false;

  async function send(keep: boolean, repeat = false) {
    const raw = text;
    const value = raw.trim();
    if (!value || !canChat || sending) return;
    // M11 A mod's `/pin <text>`: how long first, then send and pin.
    const pinned = pinText(value);
    if (pinned !== null && room.account && mods.role(room.account, room.login)) return askPin(room.account, value, pinned);
    // V107-B Text with a cheer word of this channel goes out as a cheer, never as text.
    // Rust asks (threshold, the same cheer again, pasted or recalled) or, with Bits off, says
    // it's plain text. Without the GQL sign-in, Rust's plain send decides.
    const bits = canCheer ? cheerTotal(value, isCheermote) : 0;
    if (bits > 0) {
      // A held Enter never cheers twice.
      if (repeat || askCheer) return;
      return cheer({ raw, value, bits, keep, again: false }, false, pastedCheer || recall >= 0);
    }
    return plain(raw, value, keep);
  }

  async function plain(raw: string, value: string, keep: boolean) {
    sending = true;
    try {
      const action = await chatIpc.send(room.view, value, replyTo?.id);
      error = '';
      if (action) onaction?.(action);
      sent = [value, ...sent.filter((s) => s !== value)].slice(0, HISTORY);
      recall = -1;
      // Text typed while it was sending stays; the part that was sent goes.
      if (!keep) {
        if (text.trim() === value) text = '';
        else if (text.startsWith(raw)) text = text.slice(raw.length).trimStart();
      }
      replyTo = null;
    } catch (e) {
      flash(errorText(e));
    } finally {
      sending = false;
    }
  }

  async function cheer(a: CheerAsk, confirmed: boolean, careful = false) {
    if (sending) return;
    sending = true;
    const { raw, value, keep } = a;
    /** Bits are off here: it's plain chat. */
    let asText = false;
    try {
      const r = await chatxIpc.cheer(room.view, value, a.bits, confirmed, careful);
      if (r.kind === 'ask') return void (askCheer = { ...a, again: r.again });
      asText = r.kind === 'text';
      if (r.kind === 'done') {
        cheers.setBalance(r.balance);
        error = '';
        sent = [value, ...sent.filter((s) => s !== value)].slice(0, HISTORY);
        recall = -1;
        if (!keep) {
          if (text.trim() === value) text = '';
          else if (text.startsWith(raw)) text = text.slice(raw.length).trimStart();
        }
      }
    } catch (e) {
      flash(errorText(e));
    } finally {
      sending = false;
    }
    if (asText) await plain(raw, value, keep);
  }

  /** V107-B The picker's cheer word goes into the box; the caret ends after it. */
  function pickCheer(prefix: string, amount: number) {
    text = setCheer(text, prefix, amount);
    void tick().then(() => {
      box?.focus();
      box?.setSelectionRange(text.length, text.length);
    });
  }

  /** M11 Escape keeps the text in the box. */
  function askPin(account: string, line: string, pinned: string) {
    const lengths = PIN_LENGTHS.flatMap((p) => (p.secs === null ? [] : [{ label: p.label, secs: p.secs }]));
    openMenuAt(
      { currentTarget: box ?? null },
      lengths.map((p) => ({ label: `Pin for ${p.label}`, icon: 'pin' as const, run: () => void pinSend(account, line, pinned, p.secs) })),
    );
  }

  async function pinSend(account: string, line: string, pinned: string, secs: number) {
    try {
      await modIpc.pinSend(account, room.login, pinned, secs);
      sent = [line, ...sent.filter((s) => s !== line)].slice(0, HISTORY);
      recall = -1;
      text = '';
    } catch (e) {
      flash(errorText(e));
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key !== 'Tab' && e.key !== 'Shift') cycle = null;
    if (suggest) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault();
        const n = suggest.items.length;
        suggest.index = (suggest.index + (e.key === 'ArrowDown' ? 1 : n - 1)) % n;
        return;
      }
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.ctrlKey && !suggest.arg)) {
        e.preventDefault();
        accept();
        return;
      }
      if (e.key === 'Escape') {
        suggest = null;
        return;
      }
    }
    if (e.key === 'Tab') {
      e.preventDefault();
      void tabComplete(e.shiftKey);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      void send(e.ctrlKey, e.repeat);
    } else if (e.key === 'Escape' && replyTo) {
      replyTo = null;
    } else if (e.key === 'ArrowUp' && (!text || recall >= 0) && sent.length) {
      e.preventDefault();
      recall = Math.min(recall + 1, sent.length - 1);
      text = sent[recall];
    } else if (e.key === 'ArrowDown' && recall >= 0) {
      e.preventDefault();
      recall -= 1;
      text = recall >= 0 ? sent[recall] : '';
    }
  }
</script>

<div class="input">
  {#if replyTo}
    <div class="reply-bar">
      <Icon name="reply" size={13} />
      <span>Replying to <b>@{replyTo.sender?.display}</b>: {plainText(replyTo.body)}</span>
      <button aria-label="Cancel reply" onclick={() => (replyTo = null)}><Icon name="x" size={13} /></button>
    </div>
  {/if}
  {#if suggest}
    <SuggestList items={suggest.items} index={suggest.index} onpick={accept} anchor={row} />
  {/if}
  <div class="row" bind:this={row}>
    <RewardsMenu {room} />
    {#if canChat && canCheer}<BitsButton {room} box={cheers} oninsert={pickCheer} />{/if}
    {#if canChat}
      <div class="compose">
      <textarea
        bind:this={box}
        bind:value={text}
        rows="1"
        maxlength={MAX_CHARS * 2}
        placeholder={replyTo ? 'Write a reply' : `Message #${room.login}`}
        spellcheck={settings.v.chat.spellCheck}
        data-native-menu={settings.v.chat.spellCheck ? '' : undefined}
        oninput={updateSuggest}
        onclick={updateSuggest}
        onfocus={focused}
        {onpaste}
        onblur={() => (suggest = null)}
        {onkeydown}
      ></textarea>
      {#if waitMs > 0 || chars >= SHOW_COUNT}
        <div class="meta">
          {#if waitMs > 0}<span class="wait" data-tip="You can talk again then">{Math.ceil(waitMs / 1000)}s</span>{/if}
          {#if chars >= SHOW_COUNT}<span class="count" class:over={chars > MAX} class:near={chars > MAX - 50}>{formatNumber(chars)}/{MAX}</span>{/if}
        </div>
      {/if}
      <!-- 1.43 GIFs are a tab in the emote menu. -->
      <!-- V107-Mk Badge, name color and smileys for this channel. -->
      {#if room.account && (accounts.can('badgePicker', room.account) || accounts.can('chatColor', room.account))}<IdentityButton account={room.account} login={room.login} />{/if}
      <EmoteButton login={room.login} account={room.account} {room} oninsert={(v) => void insertAtCaret(v)} />
      </div>
      {#if settings.v.chat.sendButton}
        <IconButton
          icon="send"
          label="Send"
          hint="Enter"
          disabled={!text.trim()}
          onclick={() => {
            void send(false);
            box?.focus();
          }}
        />
      {/if}
    {:else}
      <button class="signin" onclick={onsignin}>
        {room.account ? 'This account can’t chat yet. Check Settings → Accounts.' : 'Sign in to chat'}
      </button>
    {/if}
  </div>
  <!-- V1051-05 Over the box, so the box never moves. -->
  {#if up.upload.status}<p class="note uploading" role="status">{up.upload.status}</p>{/if}
  {#if error}<p class="note error" role="alert">{error}</p>{/if}
  {#if askCheer}
    <CheerConfirm
      bits={askCheer.bits}
      text={askCheer.value}
      again={askCheer.again}
      onyes={() => {
        // Read before it's cleared: the block's values go with it.
        const a = askCheer!;
        askCheer = null;
        void cheer(a, true);
      }}
      onno={() => (askCheer = null)}
    />
  {/if}
  {#if up.pending}
    <UploadConfirm
      host={up.pending.host}
      count={up.pending.count}
      pictures={up.pending.pictures}
      paths={up.pending.paths}
      names={up.pending.names}
      onyes={() => {
        const go = up.pending!.go;
        up.pending = null;
        go();
      }}
      onno={() => (up.pending = null)}
    />
  {/if}
</div>

<style>
  .input {
    position: relative;
    padding: 6px 8px 8px;
    border-top: 1px solid var(--border);
  }
  .row {
    display: flex;
    align-items: flex-end;
    gap: 6px;
  }
  /* The same box as the whisper box: the emote button inside, send outside. */
  .compose {
    flex: 1 1 0;
    min-width: 0;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    padding-right: 2px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
  }
  .compose:focus-within {
    border-color: var(--accent-border);
  }
  /* Centred on a one-line box (34 px), like the emote button inside it. */
  .row > :global(.icon-btn) {
    margin-bottom: 2px;
  }
  .compose :global(.smile) {
    height: 30px;
    margin-bottom: 1px;
  }
  .compose :global(.smile) {
    width: 30px;
  }
  textarea {
    flex: 1 1 0;
    min-width: 0;
    min-height: 32px;
    max-height: 96px;
    padding: 6px 10px;
    background: none;
    border: 0;
    outline: none;
    color: var(--text);
    resize: none;
    /* Height follows the text; the width is the row's, not the placeholder's. */
    field-sizing: content;
    overflow-x: hidden;
    font: var(--chat-size) / 1.45 var(--chat-font);
  }
  /* One line, cut with …, so a narrow chat doesn't get a two-line empty box. */
  textarea::placeholder {
    color: var(--dim);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  /* V105-11: the counter is a flex item after the text, so the text ends before it and
     can never run under it. */
  .meta {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    align-self: flex-end;
    justify-content: flex-end;
    min-height: 32px;
    padding: 0 4px 7px;
    pointer-events: none;
    font: 600 0.68rem/1.2 var(--font-mono);
    font-variant-numeric: tabular-nums;
    color: var(--dim);
    white-space: nowrap;
  }
  .meta .wait {
    color: var(--yellow);
  }
  .count.near {
    color: var(--yellow);
  }
  .count.over {
    color: var(--red);
  }
  .signin {
    flex: 1;
    height: 34px;
    background: var(--surface);
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
  }
  .signin:hover {
    color: var(--text);
    border-color: var(--accent-border);
  }
  .reply-bar {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 6px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .reply-bar span {
    flex: 1;
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .reply-bar button {
    position: relative;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .reply-bar button::before {
    content: "";
    position: absolute;
    inset: -1px;
  }
  .reply-bar button:hover {
    color: var(--text);
  }
  .note {
    position: absolute;
    right: 8px;
    bottom: calc(100% + 4px);
    left: 8px;
    z-index: var(--z-panel);
    width: fit-content;
    max-width: calc(100% - 16px);
    margin: 0;
    padding: 4px 9px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
    font-size: 0.8rem;
    pointer-events: none;
  }
  .uploading {
    color: var(--muted);
  }
  .error {
    color: var(--red);
  }
</style>
