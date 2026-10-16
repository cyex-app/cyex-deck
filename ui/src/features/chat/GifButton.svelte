<!-- 1.43 GIFs from GIPHY, for subs at Tier 2 and up where the channel allows them.
     Opens above the message box; empty search shows what's trending. A click sends. -->
<script lang="ts">
  import SearchField from '../../components/SearchField.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Gif } from '../../ipc/bindings/Gif';
  import type { GifInfo } from '../../ipc/bindings/GifInfo';
  import { chatxIpc } from '../../ipc/chatx';
  import { layer } from '../../lib/escape';
  import { firstField, focusScope } from '../../lib/focusScope';
  import { floating } from '../../lib/place';
  import { toast } from '../../lib/toast.svelte';
  import type { ChatRoom } from './room.svelte';
  import { errorText } from '../../lib/errors';
  import Loading from '../../components/Loading.svelte';

  let { room }: { room: ChatRoom } = $props();

  let open = $state(false);
  let wrap = $state<HTMLElement>();
  let info = $state<GifInfo | null>(null);
  let query = $state('');
  let gifs = $state.raw<Gif[]>([]);
  let more = $state(true);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  let pop = $state<HTMLElement>();

  // Escape clears the search first, then closes; focus returns to the GIF button.
  $effect(() => {
    if (!open || !pop) return;
    const off = layer('popover', () => (open = false));
    const release = focusScope(pop, { initial: firstField });
    const down = (e: MouseEvent) => {
      if (wrap && !wrap.contains(e.target as Node)) open = false;
    };
    addEventListener('mousedown', down, true);
    return () => {
      off();
      release();
      removeEventListener('mousedown', down, true);
    };
  });

  async function toggle() {
    open = !open;
    // A "can't send" answer asks again, in case it was a failed call.
    if (!open || info?.canSend) return;
    info = null;
    try {
      info = await chatxIpc.gifInfo(room.view);
      if (info.canSend) void load(true);
    } catch (e) {
      info = { canSend: false, reason: errorText(e) };
    }
  }

  /** A fresh search is out: "Nothing found." waits for its answer. */
  let searching = $state(false);

  async function load(fresh: boolean) {
    const q = query;
    if (fresh) searching = true;
    try {
      const page = await chatxIpc.gifSearch(room.view, q, fresh ? 0 : gifs.length);
      if (q !== query) return;
      gifs = fresh ? page : [...gifs, ...page];
      more = page.length > 0;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      if (q === query) searching = false;
    }
  }

  function oninput() {
    clearTimeout(timer);
    timer = setTimeout(() => void load(true), 300);
  }

  async function send(g: Gif) {
    if (busy) return;
    busy = true;
    try {
      const r = await chatxIpc.gifSend(room.view, g, query.trim());
      if (r.error) toast.error(r.waitSecs ? `Wait ${r.waitSecs}s before the next GIF.` : 'Couldn’t send that GIF.');
      else open = false;
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLElement;
    if (more && el.scrollHeight - el.scrollTop - el.clientHeight < 200) {
      more = false;
      void load(false);
    }
  }
</script>

<div class="wrap" bind:this={wrap}>
  <button class="gif-btn" class:on={open} aria-label="GIFs" use:tooltip={{ label: 'GIFs', hint: 'Search and send a GIF here.' }} onclick={toggle}>GIF</button>
  {#if open}
    <div class="pop" bind:this={pop} role="dialog" aria-label="GIFs" tabindex="-1" use:floating={{ anchor: wrap, side: 'above', align: 'end', gap: 8 }}>
      {#if !info}
        <Loading />
      {:else if !info.canSend}
        <p class="note">{info.reason ?? 'GIFs aren’t available here.'}</p>
      {:else}
        <SearchField bind:value={query} placeholder="Search GIPHY" small autofocus {oninput} />
        <div class="grid" {onscroll}>
          {#each gifs as g (g.id)}
            <button class="gif" disabled={busy} onclick={() => send(g)} use:tooltip={g.title}>
              <img src={g.preview} alt={g.title} loading="lazy" style:aspect-ratio="{g.width} / {g.height}" />
            </button>
          {:else}
            <p class="note">{searching ? 'Searching…' : 'Nothing found.'}</p>
          {/each}
        </div>
        <p class="by">Powered by GIPHY</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .wrap {
    position: relative;
    display: flex;
  }
  .gif-btn {
    height: 34px;
    padding: 0 6px;
    background: none;
    border: 1px solid transparent;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.72rem;
    font-weight: 800;
  }
  .gif-btn:hover,
  .gif-btn.on {
    color: var(--text);
    background: var(--surface-hover);
  }
  /* V102-03 Fixed to the window; the grid gets shorter when the window is. */
  .pop {
    position: fixed;
    z-index: var(--z-popover);
    display: flex;
    flex-direction: column;
    width: 320px;
    padding: 8px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-pop);
  }
  .grid {
    flex: 0 1 auto;
    min-height: 0;
    columns: 2;
    column-gap: 6px;
    height: 320px;
    margin-top: 8px;
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
  }
  .gif:hover {
    border-color: var(--accent-border);
  }
  .gif img {
    display: block;
    width: 100%;
  }
  .note {
    margin: 4px 2px;
    color: var(--dim);
    font-size: 0.8rem;
  }
  .by {
    margin: 6px 0 0;
    color: var(--dim);
    font-size: 0.7rem;
    text-align: right;
  }
</style>
