<!-- 1.43 GIFs from GIPHY, for subs at Tier 2 and up where the channel allows them.
     Opens above the message box; empty search shows what's trending. A click sends. -->
<script lang="ts">
  import Icon from '../../components/Icon.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { Gif } from '../../ipc/bindings/Gif';
  import type { GifInfo } from '../../ipc/bindings/GifInfo';
  import { chatxIpc } from '../../ipc/chatx';
  import { toast } from '../../lib/toast.svelte';
  import type { ChatRoom } from './room.svelte';

  let { room }: { room: ChatRoom } = $props();

  let open = $state(false);
  let wrap = $state<HTMLElement>();
  let info = $state<GifInfo | null>(null);
  let query = $state('');
  let gifs = $state.raw<Gif[]>([]);
  let more = $state(true);
  let busy = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  $effect(() => {
    if (!open) return;
    const down = (e: MouseEvent) => {
      if (wrap && !wrap.contains(e.target as Node)) open = false;
    };
    const key = (e: KeyboardEvent) => {
      if (e.key === 'Escape') open = false;
    };
    addEventListener('mousedown', down, true);
    addEventListener('keydown', key);
    return () => {
      removeEventListener('mousedown', down, true);
      removeEventListener('keydown', key);
    };
  });

  async function toggle() {
    open = !open;
    if (!open || info) return;
    try {
      info = await chatxIpc.gifInfo(room.view);
      if (info.canSend) void load(true);
    } catch (e) {
      info = { canSend: false, reason: String(e) };
    }
  }

  async function load(fresh: boolean) {
    const q = query;
    try {
      const page = await chatxIpc.gifSearch(room.view, q, fresh ? 0 : gifs.length);
      if (q !== query) return;
      gifs = fresh ? page : [...gifs, ...page];
      more = page.length > 0;
    } catch (e) {
      toast.error(String(e));
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
      if (r.error) toast.error(r.waitSecs ? `Wait ${r.waitSecs}s before the next GIF.` : `Twitch said no (${r.error}).`);
      else open = false;
    } catch (e) {
      toast.error(String(e));
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
  <button class="gif-btn" class:on={open} aria-label="GIFs" use:tooltip={'GIFs'} onclick={toggle}>GIF</button>
  {#if open}
    <div class="pop">
      {#if !info}
        <p class="note">Loading…</p>
      {:else if !info.canSend}
        <p class="note">{info.reason ?? 'GIFs aren’t available here.'}</p>
      {:else}
        <div class="bar">
          <Icon name="search" size={14} />
          <!-- svelte-ignore a11y_autofocus -->
          <input class="field" placeholder="Search GIPHY" bind:value={query} {oninput} autofocus spellcheck="false" />
        </div>
        <div class="grid" {onscroll}>
          {#each gifs as g (g.id)}
            <button class="gif" disabled={busy} onclick={() => send(g)} title={g.title}>
              <img src={g.preview} alt={g.title} loading="lazy" style:aspect-ratio="{g.width} / {g.height}" />
            </button>
          {:else}
            <p class="note">Nothing found.</p>
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
  .pop {
    position: absolute;
    right: 0;
    bottom: calc(100% + 8px);
    z-index: var(--z-popover);
    width: 320px;
    padding: 8px;
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: 0 6px 20px var(--shadow);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--dim);
  }
  .bar .field {
    flex: 1;
    height: 28px;
  }
  .grid {
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
