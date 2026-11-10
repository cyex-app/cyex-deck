<!-- F1 Search a chat with Chatterino's operators (`from:`, `has:link`, `badge:`,
     `regex:`, `is:` …), or every open chat at once (C29). Results come from Rust's
     buffers, newest at the bottom. -->
<script lang="ts">
  import Checkbox from '../../components/Checkbox.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { chatxIpc } from '../../ipc/chatx';
  import { layer } from '../../lib/escape';
  import { firstField, focusScope } from '../../lib/focusScope';
  import { actionFor, comboFromEvent } from '../../lib/hotkeys.svelte';
  import { formatNumber } from '../../lib/format';
  import { enterKey } from '../../lib/keys';
  import { openMenu } from '../../components/menu.svelte';
  import type { CardTarget } from '../emotes/emotes';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import { emoteMenu, enterClicks, lineTips, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
  import { findHits, MAX_HITS, searchCapped, searchTerms } from './searchHits';
  import Line from './Line.svelte';
  import { scrollWithin } from '../../lib/scrollWithin';
  import { onDestroy } from 'svelte';

  interface Props {
    view: number;
    channel: string;
    query?: string;
    all?: boolean;
    /** The account reading this chat, for 7TV edits from an emote's menu. */
    account?: string | null;
    onclose: () => void;
  }

  let { view, channel, query = '', all = false, account = null, onclose }: Props = $props();

  // The popup starts from what it was opened with; typing takes over from there.
  // svelte-ignore state_referenced_locally
  let text = $state(query);
  // svelte-ignore state_referenced_locally
  let everywhere = $state(all);
  let results = $state.raw<FeedLine[]>([]);
  /** A search the backend refused (a pattern that does not compile). */
  let failure = $state('');
  let root = $state<HTMLElement>();
  let list = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** A10 Where the text matched, and which hit Enter is on. */
  let hits = $state.raw<Range[]>([]);
  let cur = $state(-1);

  // Escape clears the text first, then closes; focus goes back to where the search was opened from.
  $effect(() => {
    if (!root) return;
    const off = layer('popover', onclose);
    const release = focusScope(root, { initial: firstField });
    // Ctrl+F again: the open layer holds the hotkey back, so refocus and select the field.
    const again = (e: KeyboardEvent) => {
      const id = actionFor(comboFromEvent(e) ?? '');
      if (id !== 'chat.search' && id !== 'chat.searchAll') return;
      const field = root?.querySelector<HTMLInputElement>('input[type="text"], input[type="search"], input:not([type])');
      if (!field) return;
      e.preventDefault();
      field.focus();
      field.select();
    };
    window.addEventListener('keydown', again);
    return () => {
      window.removeEventListener('keydown', again);
      off();
      release();
    };
  });

  // A10 Mark the typed words in the lines; the current hit stands out.
  $effect(() => {
    const rows = results;
    const terms = searchTerms(text);
    if (!list || !rows.length) return void (hits = []);
    const el = list;
    // After the lines are drawn.
    const id = requestAnimationFrame(() => {
      hits = findHits(el, terms);
      cur = hits.length - 1;
    });
    return () => cancelAnimationFrame(id);
  });

  $effect(() => {
    if (!('highlights' in CSS)) return;
    const now = hits[cur];
    CSS.highlights.set('search-hit', new Highlight(...hits.filter((r) => r !== now)));
    CSS.highlights.set('search-cur', new Highlight(...(now ? [now] : [])));
    const hit = now?.startContainer.parentElement;
    if (hit) scrollWithin(hit, 'center');
    return () => {
      CSS.highlights.delete('search-hit');
      CSS.highlights.delete('search-cur');
    };
  });

  /** The emote card on screen. */
  let card = $state<{ t: CardTarget; room: string } | null>(null);
  /** Lines act as in the chat (lineEvents.ts); in "All chats" each acts for its own channel. */
  function host(el: HTMLElement): LineHost {
    const room = el.closest<HTMLElement>('[data-channel]')?.dataset.channel ?? channel;
    return {
      channel: room,
      account,
      senderOf: (e) => {
        const id = e.closest<HTMLElement>('[data-mid]')?.dataset.mid;
        return results.find((r) => r.message.id === id)?.message.sender?.id;
      },
      openCard: (t) => (card = { t, room }),
      openMenu,
    };
  }
  const tips = lineTips(() => list);
  onDestroy(tips.drop);

  function onclick(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>(PARTS);
    // A name or #channel inside the row's own data-channel wrapper is not a part itself.
    if (el && !el.matches('div[data-channel]')) openPart(e, el, host(el));
  }

  function oncontextmenu(e: MouseEvent) {
    const el = (e.target as HTMLElement).closest<HTMLElement>('[data-emote]');
    if (el) void emoteMenu(e, el, host(el));
    else notEmote();
  }

  function jump(by: number) {
    if (hits.length) cur = (cur + by + hits.length) % hits.length;
  }

  function keydown(e: KeyboardEvent) {
    if (!enterKey(e)) return;
    e.preventDefault();
    jump(e.shiftKey ? 1 : -1);
  }

  $effect(() => {
    const q = text;
    const scope = everywhere ? null : view;
    clearTimeout(timer);
    let stale = false;
    timer = setTimeout(() => {
      chatxIpc
        .search(scope, q)
        .then((r) => {
          if (stale) return;
          failure = '';
          results = r.slice(-MAX_HITS);
          requestAnimationFrame(() => list && (list.scrollTop = list.scrollHeight));
        })
        .catch((e) => {
          if (stale) return;
          results = [];
          // Only the pattern refusal is worth showing; anything else reads as no match.
          failure = typeof e === 'string' && e.startsWith('Not a valid') ? e : '';
        });
    }, 150);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });
</script>

<div class="search" bind:this={root} role="dialog" aria-label="Search chat" tabindex="-1">
  <div class="bar">
    <span class="box"><SearchField bind:value={text} placeholder="Search…" label="Search messages" small autofocus onkeydown={keydown} /></span>
    {#if hits.length > 1}
      <IconButton icon="chevronUp" label="Previous match" size={14} onclick={() => jump(-1)} />
      <IconButton icon="chevronDown" label="Next match" size={14} onclick={() => jump(1)} />
    {/if}
    <span class="all"><Checkbox label="All chats" checked={everywhere} onchange={(v) => (everywhere = v)} /></span>
    <IconButton icon="x" label="Close" size={14} onclick={onclose} />
  </div>
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
  <div
    class="results"
    bind:this={list}
    {onclick}
    {oncontextmenu}
    onkeydown={enterClicks}
    onmouseover={tips.onmouseover}
    onmouseout={tips.onmouseout}
  >
    {#each results as r (`${r.channel}/${r.message.id}`)}
      <div data-channel={r.channel} data-mid={r.message.id}>
        {#if everywhere}<div class="chan">#{r.channel}</div>{/if}
        <Line m={r.message} channel={r.channel} />
      </div>
    {:else}
      <p class="muted">{text.trim() ? (failure || 'Nothing found.') : `Type to search ${everywhere ? 'every open chat' : channel}.`}</p>
    {/each}
  </div>
  {#if card}
    {#key card}<EmoteCard target={card.t} login={card.room} {account} onclose={() => (card = null)} />{/key}
  {/if}
  {#if results.length}<p class="count">{searchCapped(results.length) ? `Newest ${formatNumber(results.length)}. Type more to narrow it.` : `${formatNumber(results.length)} found`}</p>{/if}
</div>

<style>
  .search {
    position: absolute;
    inset: 0;
    z-index: var(--z-panel);
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid var(--border);
    color: var(--dim);
  }
  .box {
    flex: 1;
    min-width: 0;
  }
  .all {
    color: var(--muted);
    font-size: 0.78rem;
    white-space: nowrap;
  }
  .results {
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }
  .chan {
    padding: 4px 10px 0;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
  }
  .muted {
    padding: 8px 12px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  :global(::highlight(search-hit)) {
    background: color-mix(in srgb, var(--accent-line) 28%, transparent);
    color: inherit;
  }
  :global(::highlight(search-cur)) {
    background: color-mix(in srgb, var(--accent-line) 55%, transparent);
    color: inherit;
  }
  .count {
    margin: 0;
    padding: 4px 10px;
    border-top: 1px solid var(--border);
    color: var(--dim);
    font-size: 0.75rem;
  }
</style>
