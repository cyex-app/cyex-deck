<!-- 1.43 The GIFs tab of the emote menu: GIPHY through Twitch's own key. An empty search
     shows what's trending; a click or Enter sends the GIF on its own, as one message. -->
<script lang="ts">
  import { onDestroy, onMount, tick, untrack } from 'svelte';

  import Loading from '../../components/Loading.svelte';
  import type { Gif } from '../../ipc/bindings/Gif';
  import { chatxIpc } from '../../ipc/chatx';
  import { errorText } from '../../lib/errors';
  import { focus } from '../../lib/focus.svelte';
  import { columns } from '../../lib/gridNav';
  import { tooltip } from '../../components/tooltip.svelte';
  import { cooldownText, cooldownUntil, secsLeft, sendError, startCooldown } from './gifs';

  interface Props {
    view: number;
    /** Account and channel, for the cooldown. */
    chat: string;
    /** The menu's search box. */
    query: string;
    onsent: () => void;
  }

  let { view, chat, query, onsent }: Props = $props();

  let gifs = $state.raw<Gif[]>([]);
  let more = $state(true);
  /** A fresh search is out: "Nothing found." waits for its answer. */
  let searching = $state(true);
  /** The next page is out. */
  let paging = false;
  let busy = $state(false);
  let error = $state('');
  let until = $state(0);
  let now = $state(Date.now());
  const wait = $derived(secsLeft(until, now));
  /** fg-3 One line under the grid: an error, else the wait. */
  const line = $derived(error || (wait > 0 ? cooldownText(wait) : ''));
  /** fg-1 Two columns, each tile to the shorter one; a gap is about 6 px of a 170 px tile. */
  const cols = $derived(columns(gifs, (g) => g.height / g.width, 2, 0.035));
  let list = $state<HTMLElement>();
  /** The query the shown results are for. */
  let asked = '';
  let loaded = false;
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (wait <= 0) return;
    const t = setInterval(() => (now = Date.now()), 1000);
    return () => clearInterval(t);
  });

  // Typing waits a moment; the first load (trending) doesn't.
  $effect(() => {
    const q = query.trim();
    untrack(() => {
      clearTimeout(timer);
      if (loaded && q === asked) return;
      timer = setTimeout(() => void load(q, true), loaded ? 300 : 0);
      loaded = true;
    });
  });

  onMount(() => (until = cooldownUntil(chat)));
  onDestroy(() => clearTimeout(timer));

  async function load(q: string, fresh: boolean) {
    asked = q;
    if (fresh) searching = true;
    else paging = true;
    try {
      const page = await chatxIpc.gifSearch(view, q, fresh ? 0 : gifs.length);
      if (q !== asked) return;
      // GIPHY's pages can overlap; ids key the tiles.
      const seen = new Set(fresh ? [] : gifs.map((g) => g.id));
      const add = page.filter((g) => !seen.has(g.id) && (seen.add(g.id), true));
      gifs = fresh ? add : [...gifs, ...add];
      more = page.length > 0;
      error = '';
      if (fresh && list) list.scrollTop = 0;
      void fill();
    } catch (e) {
      if (q === asked) {
        error = errorText(e);
        // A scroll tries the page again.
        if (!fresh) more = true;
      }
    } finally {
      paging = false;
      if (q === asked) searching = false;
    }
  }

  /** fg-1 The next page when the end of the list is within 200 px, also when a page
   * doesn't fill the list and nothing can scroll. */
  async function fill() {
    await tick();
    const el = list;
    if (!el || !more || paging || searching) return;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 200) void load(asked, false);
  }

  async function send(g: Gif) {
    if (busy || wait > 0) return;
    busy = true;
    error = '';
    try {
      const r = await chatxIpc.gifSend(view, g, asked);
      now = Date.now();
      until = startCooldown(chat, r, now);
      const say = sendError(r);
      if (say) error = say;
      else if (!r.error) onsent();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

</script>

{#snippet tile(g: Gif)}
  <button
    class="gif"
    aria-label={g.title}
    aria-disabled={busy || wait > 0}
    data-tip={g.title}
    onclick={() => send(g)}
  >
    <img
      src={focus.still ? g.still : g.preview}
      alt=""
      loading="lazy"
      decoding="async"
      style:aspect-ratio="{g.width} / {g.height}"
    />
  </button>
{/snippet}

<div class="gifs">
  <!-- fg-6 A new search keeps the old tiles, dimmed, under a thin running line. -->
  <div class="grid" class:dim={searching && gifs.length > 0} bind:this={list} onscroll={fill}>
    {#if gifs.length}
      <div class="cols">
        {#each cols as col, c (c)}
          <div class="col">
            {#each col as g (g.id)}{@render tile(g)}{/each}
          </div>
        {/each}
      </div>
    {:else if searching}
      <Loading />
    {:else if !error}
      <p class="note">Nothing found.</p>
    {/if}
  </div>
  {#if searching && gifs.length}<div class="run" aria-hidden="true"></div>{/if}
  <div class="foot">
    <!-- fg-3 Always there, so the grid never moves; an error replaces the wait. -->
    <p class="line" class:error={!!error} role={error ? 'alert' : 'status'} use:tooltip={line}>{line}</p>
    <p class="by">Powered by GIPHY</p>
  </div>
</div>

<style>
  .gifs {
    position: relative;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .grid {
    flex: 1;
    min-height: 0;
    padding: 6px 8px;
    overflow: hidden auto;
    overscroll-behavior: contain;
    /* fg-5 Room for a focus ring at the edges. */
    scroll-padding: 6px 0;
  }
  .cols {
    display: flex;
    gap: 6px;
    align-items: flex-start;
    transition: opacity var(--fast) ease-out;
  }
  /* A quick answer never flickers. */
  .dim .cols {
    opacity: 0.6;
    transition-delay: var(--fast);
  }
  .col {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex: 1;
    min-width: 0;
  }
  .gif {
    display: block;
    width: 100%;
    padding: 0;
    background: var(--surface);
    border: 1px solid transparent;
    border-radius: var(--radius);
    overflow: hidden;
    transition: transform var(--fast) ease-out;
  }
  .gif:hover:not([aria-disabled='true']),
  .gif:focus-visible {
    border-color: var(--accent-border);
  }
  .gif:active:not([aria-disabled='true']) {
    transform: scale(0.97);
  }
  .gif[aria-disabled='true'] {
    cursor: default;
    opacity: 0.6;
  }
  .gif img {
    display: block;
    width: 100%;
  }
  /* fg-6 Over the grid's top edge, out of the layout. */
  .run {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 2px;
    overflow: hidden;
    pointer-events: none;
    opacity: 0;
    animation: appear 1ms var(--fast) forwards;
  }
  @keyframes appear {
    to {
      opacity: 1;
    }
  }
  .run::after {
    content: '';
    position: absolute;
    inset: 0 auto 0 0;
    width: 30%;
    background: var(--muted);
    opacity: 0.6;
    animation: run 900ms ease-in-out infinite;
  }
  @keyframes run {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(340%);
    }
  }
  .foot {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 6px;
  }
  .line,
  .by {
    margin: 0;
    line-height: 18px;
    white-space: nowrap;
  }
  .line {
    flex: 1;
    min-width: 0;
    /* Same height empty or not. */
    min-height: 18px;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .line.error {
    color: var(--red);
  }
  .note {
    margin: 16px 4px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .by {
    color: var(--dim);
    font-size: 0.7rem;
  }
  @media (prefers-reduced-motion: reduce) {
    .gif,
    .cols {
      transition: none;
    }
    .run::after {
      animation: none;
      width: 100%;
    }
  }
</style>
