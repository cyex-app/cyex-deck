<!-- Message box (1.8, C6, C21): `@`, `:` and `!` suggestions (emotes, emoji E1, bot
     commands C13), Tab completion, the emote menu (1.40), length counter, slow mode and
     timeout countdown, replies. Enter sends; Ctrl+Enter sends and keeps the text for
     sending again. Up recalls what you sent. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import { chatIpc } from '../../ipc/chat';
  import { modIpc } from '../../ipc/moderation';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { UiAction } from '../../ipc/bindings/UiAction';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { complete, insert, wordAt, type Word } from './completion';
  import { commandAt, commandMatches } from './commandTab';
  import { chatxIpc } from '../../ipc/chatx';
  import { pluginsIpc } from '../../ipc/plugins';
  import { mods, PIN_LENGTHS } from '../modview/moderation.svelte';
  import { plugins } from '../plugins/plugins.svelte';
  import EmoteButton from './EmoteButton.svelte';
  import GifButton from './GifButton.svelte';
  import RewardsMenu from './RewardsMenu.svelte';
  import type { ChatRoom } from './room.svelte';
  import { Sources } from './sources.svelte';
  import SuggestList from './SuggestList.svelte';
  import type { Item } from './suggestions';
  import { pinText } from './pinSend';
  import { plainText } from './text';

  interface Props {
    room: ChatRoom;
    replyTo: ChatMessage | null;
    onsignin: () => void;
    /** What a command asked the window to do (S1). */
    onaction?: (a: UiAction) => void;
  }

  let { room, replyTo = $bindable(), onsignin, onaction }: Props = $props();

  const MAX = 500;
  const HISTORY = 20;

  let box = $state<HTMLTextAreaElement>();
  /** The message box with its buttons: suggestions line up with it (V102-03). */
  let row = $state<HTMLElement>();
  let text = $state('');
  let error = $state('');
  let sent: string[] = [];
  let recall = -1;
  let now = $state(Date.now());
  /** 1.43 Shown once Twitch says this account can send GIFs here; asked on first focus. */
  let gifs = $state<boolean | null>(null);

  interface Suggest {
    word: Word;
    items: Item[];
    index: number;
  }
  let suggest = $state<Suggest | null>(null);
  /** Tab cycling through matches for one word. */
  let cycle: { word: Word; items: string[]; index: number } | null = null;
  /** S5 Plugins are being asked for matches. */
  let asking = false;

  const canChat = $derived(room.account !== null && accounts.can('sendChat', room.account));
  const chars = $derived([...text].length);
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

  function updateSuggest() {
    const word = wordAt(text, box?.selectionStart ?? text.length);
    const items = sources.items(word);
    suggest = items.length ? { word, items, index: 0 } : null;
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
    if (gifs !== null || !room.view) return;
    gifs = false;
    chatxIpc
      .gifInfo(room.view)
      .then((i) => (gifs = i.canSend))
      .catch(() => {});
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
    if (asking) return;
    const view = room.view;
    const own = async (): Promise<string[]> => {
      if (command !== null) {
        if (!command || !view) return [];
        return commandMatches(command, await chatIpc.commands(view).catch(() => []));
      }
      if (!bare) return [];
      const people = sources.chatters().map((n) => (word.text.startsWith('@') ? `@${n}` : n));
      return complete(bare, [...sources.emotes().map((e) => e.name), ...people], 30, sources.used());
    };
    const all = async () => {
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
    void put(word, items[0]);
    cycle = { word: { start: word.start, end: word.start + items[0].length, text: items[0] }, items, index: 0 };
  }

  async function send(keep: boolean) {
    const value = text.trim();
    if (!value || !canChat) return;
    // M11 A mod's `/pin <text>`: how long first, then send and pin.
    const pinned = pinText(value);
    if (pinned !== null && room.account && mods.role(room.account, room.login)) return askPin(room.account, value, pinned);
    try {
      const action = await chatIpc.send(room.view, value, replyTo?.id);
      error = '';
      if (action) onaction?.(action);
      sent = [value, ...sent.filter((s) => s !== value)].slice(0, HISTORY);
      recall = -1;
      if (!keep) text = '';
      replyTo = null;
    } catch (e) {
      error = String(e);
      setTimeout(() => (error = ''), 4000);
    }
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
      error = String(e);
      setTimeout(() => (error = ''), 4000);
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
      if (e.key === 'Tab' || (e.key === 'Enter' && !e.ctrlKey)) {
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
      void send(e.ctrlKey);
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

<div class="input" class:gifs>
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
    {#if canChat}
      <textarea
        bind:this={box}
        bind:value={text}
        rows="1"
        maxlength={MAX * 2}
        placeholder={replyTo ? 'Write a reply' : `Message #${room.login}`}
        spellcheck={settings.v.chat.spellCheck}
        data-native-menu={settings.v.chat.spellCheck ? '' : undefined}
        oninput={updateSuggest}
        onclick={updateSuggest}
        onfocus={focused}
        onblur={() => (suggest = null)}
        {onkeydown}
      ></textarea>
      {#if gifs}<GifButton {room} />{/if}
      <EmoteButton login={room.login} account={room.account} oninsert={(v) => void insertAtCaret(v)} />
      <div class="meta">
        {#if waitMs > 0}<span class="wait" data-tip="You can talk again then">{Math.ceil(waitMs / 1000)}s</span>{/if}
        {#if chars > 0}<span class="count" class:over={chars > MAX} class:near={chars > MAX - 50}>{chars}/{MAX}</span>{/if}
      </div>
    {:else}
      <button class="signin" onclick={onsignin}>
        {room.account ? 'This account can’t chat yet. Check Settings → Accounts.' : 'Sign in to chat'}
      </button>
    {/if}
  </div>
  {#if error}<p class="error">{error}</p>{/if}
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
  textarea {
    flex: 1;
    min-height: 34px;
    max-height: 96px;
    padding: 7px 10px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    outline: none;
    resize: none;
    field-sizing: content;
    font: var(--chat-size) / 1.45 var(--chat-font);
  }
  textarea:focus {
    border-color: var(--accent-border);
  }
  textarea::placeholder {
    color: var(--dim);
  }
  .meta {
    position: absolute;
    right: 56px;
    bottom: 15px;
    display: flex;
    gap: 8px;
    pointer-events: none;
    font: 600 0.72rem var(--font-mono);
    color: var(--dim);
  }
  .gifs .meta {
    right: 96px;
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
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    color: var(--dim);
  }
  .reply-bar button:hover {
    color: var(--text);
  }
  .error {
    margin: 6px 2px 0;
    color: var(--red);
    font-size: 0.8rem;
  }
</style>
