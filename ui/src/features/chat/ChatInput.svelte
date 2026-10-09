<!-- Message box (1.8, C6, C21): `@` and `:` suggestions, Tab completion, length counter,
     slow mode and timeout countdown, replies. Enter sends; Ctrl+Enter sends and keeps
     the text for sending again. Up recalls what you sent. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { chatIpc } from '../../ipc/chat';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { Provider } from '../../ipc/bindings/Provider';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { complete, insert, wordAt, type Word } from './completion';
  import RewardsMenu from './RewardsMenu.svelte';
  import type { ChatRoom } from './room.svelte';
  import { plainText } from './text';
  import { emoteImg } from './urls';

  interface Props {
    room: ChatRoom;
    replyTo: ChatMessage | null;
    onsignin: () => void;
  }

  let { room, replyTo = $bindable(), onsignin }: Props = $props();

  const MAX = 500;
  const HISTORY = 20;

  let box = $state<HTMLTextAreaElement>();
  let text = $state('');
  let error = $state('');
  let sent: string[] = [];
  let recall = -1;
  let now = $state(Date.now());

  interface Suggest {
    word: Word;
    items: string[];
    index: number;
    /** `:` shows emote pictures. */
    emotes: boolean;
  }
  let suggest = $state<Suggest | null>(null);
  /** Tab cycling through matches for one word. */
  let cycle: { word: Word; items: string[]; index: number } | null = null;

  const canChat = $derived(room.account !== null && accounts.can('sendChat', room.account));
  const chars = $derived([...text].length);
  const waitMs = $derived(room.state.waitUntil ? room.state.waitUntil - now : 0);
  const theme = $derived(settings.v.appearance.theme);

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

  /** Recent chatters, newest first. */
  function chatters(): string[] {
    const names: string[] = [];
    const seen = new Set<string>();
    for (let i = room.messages.length - 1; i >= 0 && names.length < 500; i--) {
      const s = room.messages[i].sender;
      if (s && !seen.has(s.id)) {
        seen.add(s.id);
        names.push(s.display);
      }
    }
    return names;
  }

  /** Emotes seen in this chat, by name. Phase 7 adds the providers' sets. */
  function emotes(): Map<string, { provider: Provider; id: string }> {
    const map = new Map<string, { provider: Provider; id: string }>();
    for (const m of room.messages) {
      for (const f of m.body) if (f.t === 'emote') map.set(f.name, { provider: f.provider, id: f.id });
    }
    return map;
  }

  let emoteMap = $state.raw(new Map<string, { provider: Provider; id: string }>());

  function updateSuggest() {
    const caret = box?.selectionStart ?? text.length;
    const word = wordAt(text, caret);
    if (word.text.startsWith('@') && word.text.length >= 2) {
      const items = complete(word.text.slice(1), chatters()).map((n) => `@${n}`);
      suggest = items.length ? { word, items, index: 0, emotes: false } : null;
    } else if (word.text.startsWith(':') && word.text.length >= 3) {
      emoteMap = emotes();
      const items = complete(word.text.slice(1), emoteMap.keys());
      suggest = items.length ? { word, items, index: 0, emotes: true } : null;
    } else {
      suggest = null;
    }
  }

  async function put(word: Word, value: string) {
    const next = insert(text, word, value);
    text = next.text;
    await tick();
    box?.setSelectionRange(next.caret, next.caret);
  }

  function accept(i = suggest?.index ?? 0) {
    if (!suggest) return;
    void put(suggest.word, suggest.items[i]);
    suggest = null;
  }

  /** C6 Tab: complete the word, Tab again for the next match. */
  function tabComplete(back: boolean) {
    if (cycle) {
      const n = cycle.items.length;
      cycle.index = (cycle.index + (back ? n - 1 : 1)) % n;
      const value = cycle.items[cycle.index];
      const word = cycle.word;
      void put(word, value);
      cycle.word = { start: word.start, end: word.start + value.length, text: value };
      return;
    }
    const word = wordAt(text, box?.selectionStart ?? text.length);
    const bare = word.text.replace(/^@/, '');
    if (!bare) return;
    const people = chatters().map((n) => (word.text.startsWith('@') ? `@${n}` : n));
    const items = complete(bare, [...emotes().keys(), ...people], 30);
    if (!items.length) return;
    void put(word, items[0]);
    cycle = { word: { start: word.start, end: word.start + items[0].length, text: items[0] }, items, index: 0 };
  }

  async function send(keep: boolean) {
    const value = text.trim();
    if (!value || !canChat) return;
    try {
      await chatIpc.send(room.view, value, replyTo?.id);
      error = '';
      sent = [value, ...sent.filter((s) => s !== value)].slice(0, HISTORY);
      recall = -1;
      if (!keep) text = '';
      replyTo = null;
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
      tabComplete(e.shiftKey);
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

<div class="input">
  {#if replyTo}
    <div class="reply-bar">
      <Icon name="reply" size={13} />
      <span>Replying to <b>@{replyTo.sender?.display}</b>: {plainText(replyTo.body)}</span>
      <button aria-label="Cancel reply" onclick={() => (replyTo = null)}><Icon name="x" size={13} /></button>
    </div>
  {/if}
  {#if suggest}
    <div class="suggest" role="listbox">
      {#each suggest.items as item, i (item)}
        <button
          class="item"
          class:on={i === suggest.index}
          role="option"
          aria-selected={i === suggest.index}
          onmousedown={(e) => {
            e.preventDefault();
            accept(i);
          }}
        >
          {#if suggest.emotes}
            {@const e = emoteMap.get(item)}
            {#if e}<img src={emoteImg(e.provider, e.id, theme).src} alt="" />{/if}
          {/if}
          {item}
        </button>
      {/each}
    </div>
  {/if}
  <div class="row">
    <RewardsMenu {room} />
    {#if canChat}
      <textarea
        bind:this={box}
        bind:value={text}
        rows="1"
        maxlength={MAX * 2}
        placeholder={replyTo ? 'Write a reply' : `Message #${room.login}`}
        spellcheck="false"
        oninput={updateSuggest}
        onclick={updateSuggest}
        onblur={() => (suggest = null)}
        {onkeydown}
      ></textarea>
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
    right: 16px;
    bottom: 15px;
    display: flex;
    gap: 8px;
    pointer-events: none;
    font: 600 0.72rem var(--font-mono);
    color: var(--dim);
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
  .suggest {
    position: absolute;
    bottom: calc(100% - 2px);
    left: 8px;
    right: 8px;
    z-index: var(--z-popover);
    max-height: 260px;
    overflow-y: auto;
    padding: 4px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 8px 24px var(--shadow);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    height: 30px;
    padding: 0 8px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
  }
  .item img {
    height: 22px;
  }
  .item.on,
  .item:hover {
    background: var(--accent-dim);
  }
  .error {
    margin: 6px 2px 0;
    color: var(--red);
    font-size: 0.8rem;
  }
</style>
