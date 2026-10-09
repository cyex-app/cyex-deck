<!-- P8 A VOD's chat in step with the video. Twitch hands out about 50 lines from a point
     in time, so they're fetched a little ahead as the video plays, and again after a seek. -->
<script lang="ts">
  import { tick, type Snippet } from 'svelte';

  import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
  import { playerIpc } from '../../ipc/player';
  import { catalogs } from './catalog.svelte';
  import Line from './Line.svelte';

  interface Props {
    videoId: string;
    /** Where the video is, in seconds. */
    time: number;
    login: string;
    name: string;
    onlive: () => void;
    actions?: Snippet;
    onheaderdown?: (e: PointerEvent) => void;
  }

  let { videoId, time, login, name, onlive, actions, onheaderdown }: Props = $props();

  // svelte-ignore state_referenced_locally
  void catalogs.loadChannel(login);
  void catalogs.loadGlobal();

  interface Item {
    m: ChatMessage;
    at: number;
  }

  let items = $state.raw<Item[]>([]);
  let from = 0;
  /** Up to where lines are loaded, in seconds. */
  let to = -1;
  let loading = $state(false);
  let list = $state<HTMLElement>();
  let following = $state(true);

  async function load(offset: number, fresh: boolean) {
    loading = true;
    const at = Math.max(0, Math.floor(offset));
    try {
      const got = (await playerIpc.comments(videoId, at)).map((m) => ({ m, at: Number(m.id.split('@')[0]) }));
      if (fresh) {
        items = got;
        from = at;
      } else {
        const seen = new Set(items.map((i) => i.m.id));
        items = [...items, ...got.filter((i) => !seen.has(i.m.id))].slice(-600);
      }
      const newest = got.length ? got[got.length - 1].at : at + 30;
      // A second packed with lines: move on rather than ask for it again.
      to = Math.max(newest, at + 1);
    } catch {
      to = at + 30;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    const t = time;
    if (loading) return;
    if (to < 0 || t < from - 2 || t > to + 60) void load(t, true);
    else if (t > to - 20) void load(to, false);
  });

  const shown = $derived(items.filter((i) => i.at <= time).slice(-150));

  $effect(() => {
    void shown;
    if (following) void tick().then(() => list && (list.scrollTop = list.scrollHeight));
  });

  function onscroll() {
    if (!list) return;
    following = list.scrollTop + list.clientHeight >= list.scrollHeight - 24;
  }
</script>

<section class="replay">
  <!-- The header drags a tile in a tiled tab (U3); its buttons stay clickable. -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <header onpointerdown={onheaderdown}>
    <span class="title">{name}</span>
    <span class="tag">Chat replay</span>
    <button class="switch" onclick={onlive}>Live chat</button>
    {@render actions?.()}
  </header>
  <div class="list" bind:this={list} {onscroll}>
    {#each shown as i (i.m.id)}
      <Line m={i.m} channel={login} />
    {:else}
      <p class="empty">{loading ? 'Loading chat…' : 'No chat here yet.'}</p>
    {/each}
  </div>
</section>

<style>
  .replay {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--bg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 36px;
    padding: 0 6px 0 12px;
    border-bottom: 1px solid var(--border);
  }
  .title {
    font-weight: 700;
  }
  .tag {
    flex: 1;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .switch {
    padding: 2px 8px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.75rem;
    font-weight: 600;
  }
  .switch:hover {
    color: var(--text);
  }
  .list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 4px 0;
    font-family: var(--chat-font);
    font-size: var(--chat-size);
  }
  .empty {
    margin: 16px;
    color: var(--dim);
    text-align: center;
  }
</style>
