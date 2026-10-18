<!-- 1.43 The GIFs tab of the emote menu: GIPHY through Twitch's own key. An empty search
     shows what's trending; a click or Enter sends the GIF on its own, as one message. -->
<script lang="ts">
  import { onDestroy, onMount, untrack } from 'svelte';

  import Loading from '../../components/Loading.svelte';
  import type { Gif } from '../../ipc/bindings/Gif';
  import { chatxIpc } from '../../ipc/chatx';
  import { errorText } from '../../lib/errors';
  import { focus } from '../../lib/focus.svelte';
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
  let busy = $state(false);
  let error = $state('');
  let until = $state(0);
  let now = $state(Date.now());
  const wait = $derived(secsLeft(until, now));
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
    try {
      const page = await chatxIpc.gifSearch(view, q, fresh ? 0 : gifs.length);
      if (q !== asked) return;
      gifs = fresh ? page : [...gifs, ...page];
      more = page.length > 0;
      error = '';
    } catch (e) {
      if (q === asked) error = errorText(e);
    } finally {
      if (q === asked) searching = false;
    }
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

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (more && !searching && el.scrollHeight - el.scrollTop - el.clientHeight < 200) {
      more = false;
      void load(asked, false);
    }
  }
</script>

<div class="gifs">
  {#if wait > 0}<p class="wait" role="status">{cooldownText(wait)}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="grid" {onscroll}>
    {#each gifs as g (g.id)}
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
    {:else}
      {#if searching}<Loading />{:else if !error}<p class="note">Nothing found.</p>{/if}
    {/each}
  </div>
  <p class="by">Powered by GIPHY</p>
</div>

<style>
  .gifs {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }
  .grid {
    flex: 1;
    min-height: 0;
    columns: 2;
    column-gap: 6px;
    padding: 6px 8px 0;
    overflow-y: auto;
  }
  .gif {
    display: block;
    width: 100%;
    margin-bottom: 6px;
    padding: 0;
    background: var(--surface);
    border: 1px solid transparent;
    border-radius: var(--radius);
    overflow: hidden;
    break-inside: avoid;
    transition: transform 100ms ease-out;
  }
  .gif:hover,
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
  .wait,
  .error {
    margin: 6px 8px 0;
    font-size: 0.8rem;
  }
  .wait {
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .error {
    color: var(--red);
  }
  .note {
    margin: 16px 4px;
    color: var(--dim);
    font-size: 0.85rem;
  }
  .by {
    margin: 0;
    padding: 4px 8px 6px;
    color: var(--dim);
    font-size: 0.7rem;
    text-align: right;
  }
  @media (prefers-reduced-motion: reduce) {
    .gif {
      transition: none;
    }
  }
</style>
