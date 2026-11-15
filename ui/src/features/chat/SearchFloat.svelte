<!-- V111-33 F1 Search a chat (or every open chat, C29) in a window of its own. It opens on the
     chat as it was then (a snapshot); the chat underneath keeps moving. Typing searches that
     snapshot with Chatterino's operators (`from:`, `has:link`, `badge:`, `regex:`, `is:` …),
     results come from Rust's buffers, cut at the snapshot time. Refresh takes a new snapshot. -->
<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  import Checkbox from '../../components/Checkbox.svelte';
  import FloatPanel from '../../components/FloatPanel.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { openMenu } from '../../components/menu.svelte';
  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import type { FeedLine } from '../../ipc/bindings/FeedLine';
  import { chatxIpc } from '../../ipc/chatx';
  import { formatNumber, formatTime } from '../../lib/format';
  import { actionFor, comboFromEvent } from '../../lib/hotkeys.svelte';
  import { enterKey } from '../../lib/keys';
  import { scrollWithin } from '../../lib/scrollWithin';
  import type { CardTarget } from '../emotes/emotes';
  import EmoteCard from '../emotes/EmoteCard.svelte';
  import Line from './Line.svelte';
  import { emoteMenu, enterClicks, lineTips, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
  import { cutoffOf, findHits, MAX_HITS, searchCapped, searchTerms, upTo } from './searchHits';

  interface Props {
    view: number;
    channel: string;
    query?: string;
    all?: boolean;
    /** The account reading this chat, for 7TV edits from an emote's menu. */
    account?: string | null;
    /** The chat's lines right now; read when the window opens and on Refresh. */
    lines: () => ChatMessage[];
    nth?: number;
    onclose: () => void;
  }

  let { view, channel, query = '', all = false, account = null, lines, nth = 0, onclose }: Props = $props();

  // svelte-ignore state_referenced_locally
  let text = $state(query);
  // svelte-ignore state_referenced_locally
  let everywhere = $state(all);
  /** The chat as it was: its lines and when that was. */
  function take() {
    const now = lines();
    return { rows: now.slice(-MAX_HITS).map((message): FeedLine => ({ channel, message, global: false })), at: cutoffOf(now, Date.now()) };
  }
  let snap = $state.raw(take());
  let found = $state.raw<FeedLine[]>([]);
  /** A search the backend refused (a pattern that does not compile). */
  let failure = $state('');
  let panel = $state<FloatPanel>();
  let list = $state<HTMLElement>();
  let field = $state<SearchField>();
  let timer: ReturnType<typeof setTimeout> | undefined;
  /** A10 Where the text matched, and which hit Enter is on. */
  let hits = $state.raw<Range[]>([]);
  /** The line a hit is in. */
  const lineOf = (r: Range) => r.startContainer.parentElement?.closest('[data-mid]') ?? null;
  const linesOf = (rs: Range[]) => [...new Set(rs.map(lineOf).filter((l): l is Element => !!l))];
  const lineCount = (rs: Range[]) => linesOf(rs).length;
  /** The lines with a hit, oldest first. */
  const hitLines = $derived(linesOf(hits));
  let cur = $state(-1);

  const searching = $derived(text.trim().length > 0);
  const results = $derived(searching ? found : snap.rows);
  // svelte-ignore state_referenced_locally
  const id = `sw${view}`;

  export function focus() {
    panel?.focus();
    field?.focus();
  }
  /** Ctrl+F again, or `/search words`: the window takes the text. */
  export function set(q: string, every: boolean) {
    if (q) text = q;
    everywhere = every;
    focus();
  }

  function refresh() {
    snap = take();
    scrollEnd();
    field?.focus();
  }

  function scrollEnd() {
    requestAnimationFrame(() => list && (list.scrollTop = list.scrollHeight));
  }

  onMount(() => {
    scrollEnd();
    // Ctrl+F in the window: the open one takes the key, so select the field.
    const again = (e: KeyboardEvent) => {
      const a = actionFor(comboFromEvent(e) ?? '');
      if (a !== 'chat.search' && a !== 'chat.searchAll') return;
      if (!list?.closest('[role="dialog"]')?.contains(document.activeElement)) return;
      e.preventDefault();
      field?.focus();
    };
    window.addEventListener('keydown', again);
    // Each window marks its own words: the highlight names are page-wide.
    const sheet = document.createElement('style');
    sheet.textContent =
      `::highlight(${id}-hit){background:color-mix(in srgb,var(--accent-line) 28%,transparent);color:inherit}` +
      `::highlight(${id}-cur){background:color-mix(in srgb,var(--accent-line) 55%,transparent);color:inherit}`;
    document.head.append(sheet);
    return () => {
      window.removeEventListener('keydown', again);
      sheet.remove();
    };
  });

  // A10 Mark the typed words in the lines; the current hit stands out.
  $effect(() => {
    const rows = results;
    const terms = searchTerms(text);
    if (!list || !rows.length) return void (hits = []);
    const el = list;
    // After the lines are drawn.
    const raf = requestAnimationFrame(() => {
      hits = findHits(el, terms);
      cur = lineCount(hits) - 1;
    });
    return () => cancelAnimationFrame(raf);
  });

  $effect(() => {
    if (!('highlights' in CSS)) return;
    // V111-33 Counted and stepped by line: all of the current line's words stand out.
    const line = hitLines[cur];
    const now = hits.filter((r) => lineOf(r) === line);
    CSS.highlights.set(`${id}-hit`, new Highlight(...hits.filter((r) => lineOf(r) !== line)));
    CSS.highlights.set(`${id}-cur`, new Highlight(...(line ? now : [])));
    const hit = now[0]?.startContainer.parentElement;
    if (hit) scrollWithin(hit, 'center');
    return () => {
      CSS.highlights.delete(`${id}-hit`);
      CSS.highlights.delete(`${id}-cur`);
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
        const mid = e.closest<HTMLElement>('[data-mid]')?.dataset.mid;
        return results.find((r) => r.message.id === mid)?.message.sender?.id;
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
    if (hitLines.length) cur = (cur + by + hitLines.length) % hitLines.length;
  }

  /** Enter and ArrowUp go to the older match, Shift+Enter and ArrowDown to the newer. */
  function keydown(e: KeyboardEvent) {
    if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
      e.preventDefault();
      jump(e.key === 'ArrowUp' ? -1 : 1);
    } else if (enterKey(e)) {
      e.preventDefault();
      jump(e.shiftKey ? 1 : -1);
    }
  }

  $effect(() => {
    const q = text;
    const scope = everywhere ? null : view;
    const cut = snap.at;
    clearTimeout(timer);
    if (!q.trim()) {
      found = [];
      failure = '';
      return;
    }
    let stale = false;
    timer = setTimeout(() => {
      chatxIpc
        .search(scope, q)
        .then((r) => {
          if (stale) return;
          failure = '';
          found = upTo(r, cut).slice(-MAX_HITS);
          scrollEnd();
        })
        .catch((e) => {
          if (stale) return;
          found = [];
          // Only the pattern refusal is worth showing; anything else reads as no match.
          failure = typeof e === 'string' && e.startsWith('Not a valid') ? e : '';
        });
    }, 150);
    return () => {
      stale = true;
      clearTimeout(timer);
    };
  });

  const status = $derived.by(() => {
    if (!results.length) return '';
    if (hitLines.length) return `${formatNumber(cur + 1)} of ${formatNumber(hitLines.length)}`;
    return searching && searchCapped(results.length) ? `Newest ${formatNumber(results.length)}` : `${formatNumber(results.length)} ${searching ? 'found' : 'lines'}`;
  });
</script>

<FloatPanel bind:this={panel} label="Search #{channel}" {nth} width={440} height={520} min={{ w: 320, h: 260 }} place="search:{view}" {onclose}>
  <Card title="Search #{channel}" flush>
    {#snippet tools()}
      <span class="all"><Checkbox label="All chats" checked={everywhere} onchange={(v) => (everywhere = v)} /></span>
      <IconButton icon="refresh" label="Refresh" hint="Takes a new snapshot with the lines since." size={14} onclick={refresh} />
    {/snippet}
    <div class="wrap">
      <div class="bar">
        <span class="box"><SearchField bind:this={field} bind:value={text} placeholder="Search…" label="Search messages" small autofocus onkeydown={keydown} /></span>
        {#if hitLines.length > 1}
          <IconButton icon="chevronUp" label="Previous match" size={14} onclick={() => jump(-1)} />
          <IconButton icon="chevronDown" label="Next match" size={14} onclick={() => jump(1)} />
        {/if}
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions, a11y_mouse_events_have_key_events -->
      <div class="results" bind:this={list} {onclick} {oncontextmenu} onkeydown={enterClicks} onmouseover={tips.onmouseover} onmouseout={tips.onmouseout}>
        {#each results as r (`${r.channel}/${r.message.id}`)}
          <div data-channel={r.channel} data-mid={r.message.id}>
            {#if everywhere && searching}<div class="chan">#{r.channel}</div>{/if}
            <Line m={r.message} channel={r.channel} />
          </div>
        {:else}
          <p class="muted">{searching ? failure || 'Nothing found.' : 'No lines yet.'}</p>
        {/each}
      </div>
      {#if card}
        {#key card}<EmoteCard target={card.t} login={card.room} {account} onclose={() => (card = null)} />{/key}
      {/if}
      <p class="count">
        <span>{status}</span>
        <span class="snap">Chat as of {formatTime(new Date(snap.at), { seconds: true })}</span>
      </p>
    </div>
  </Card>
</FloatPanel>

<style>
  .wrap {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
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
    min-height: 0;
    overflow-y: auto;
    padding: 4px 0;
    background: var(--bg);
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
  .count {
    display: flex;
    justify-content: space-between;
    gap: 8px;
    margin: 0;
    padding: 4px 22px 4px 10px;
    border-top: 1px solid var(--border);
    color: var(--dim);
    font-size: 0.75rem;
  }
  .snap {
    font-variant-numeric: tabular-nums;
  }
</style>
