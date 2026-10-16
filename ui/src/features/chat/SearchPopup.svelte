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
  import { lineClick } from './clicks';
  import { findHits, searchTerms } from './searchHits';
  import Line from './Line.svelte';
  import { scrollWithin } from '../../lib/scrollWithin';

  interface Props {
    view: number;
    channel: string;
    query?: string;
    all?: boolean;
    onclose: () => void;
  }

  let { view, channel, query = '', all = false, onclose }: Props = $props();

  // The popup starts from what it was opened with; typing takes over from there.
  // svelte-ignore state_referenced_locally
  let text = $state(query);
  // svelte-ignore state_referenced_locally
  let everywhere = $state(all);
  let results = $state.raw<FeedLine[]>([]);
  let root = $state<HTMLElement>();
  let list = $state<HTMLElement>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** A10 Where the text matched, and which hit Enter is on. */
  let hits = $state.raw<Range[]>([]);
  let cur = $state(-1);
  /** Newest hits drawn; a longer list would be a wall of full lines. */
  const MAX_HITS = 300;

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

  function jump(by: number) {
    if (hits.length) cur = (cur + by + hits.length) % hits.length;
  }

  function keydown(e: KeyboardEvent) {
    if (e.key !== 'Enter' || e.isComposing) return;
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
          results = r.slice(-MAX_HITS);
          requestAnimationFrame(() => list && (list.scrollTop = list.scrollHeight));
        })
        .catch(() => !stale && (results = []));
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
  <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
  <div class="results" bind:this={list} onclick={lineClick}>
    {#each results as r (`${r.channel}/${r.message.id}`)}
      <div data-channel={r.channel}>
        {#if everywhere}<div class="chan">#{r.channel}</div>{/if}
        <Line m={r.message} channel={r.channel} />
      </div>
    {:else}
      <p class="muted">{text.trim() ? 'Nothing found.' : `Type to search ${everywhere ? 'every open chat' : channel}.`}</p>
    {/each}
  </div>
  {#if results.length}<p class="count">{formatNumber(results.length)} found</p>{/if}
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
    background: color-mix(in srgb, var(--accent) 28%, transparent);
    color: inherit;
  }
  :global(::highlight(search-cur)) {
    background: color-mix(in srgb, var(--accent) 55%, transparent);
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
