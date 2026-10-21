<!-- V107-R39 Video producer: your past broadcasts, highlights and uploads, unpublished ones
     too. A video plays in the app's VOD player; its ⋯ renames it (in the row), publishes or
     unpublishes it, copies the link or deletes it (a second click). -->
<script lang="ts">
  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import Card from '../../components/PanelCard.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { ManagedVideo } from '../../ipc/bindings/ManagedVideo';
  import { contentIpc } from '../../ipc/content';
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import ContentPlayer from './ContentPlayer.svelte';
  import { appended, count, kept, type KindPick, kindOf, length, type Playing, VIDEO_KINDS, videoLine, videoLink, videoTag, toggled } from './content';
  import { sm } from './streammanager.svelte';

  /** Twitch's limit for a video title. */
  const TITLE_MAX = 140;

  let kind = $state<KindPick>('all');
  let videos = $state<ManagedVideo[] | null>(null);
  let cursor = $state<string | null>(null);
  let total = $state<number | null>(null);
  let problem = $state<string | null>(null);
  let more = $state(false);
  let playing = $state<Playing | null>(null);
  let reload = $state(0);
  /** The row being renamed and its text. */
  let editing = $state<{ id: string; text: string } | null>(null);
  /** Rows with a write on the way. */
  let working = $state<Set<string>>(new Set());
  /** Ticked rows, for Publish, Unpublish or Delete at once. */
  let picked = $state<Set<string>>(new Set());
  let bulk = $state(false);
  const all = $derived(!!videos?.length && picked.size === videos.length);

  $effect(() => {
    const k = kindOf(kind);
    void reload;
    let alive = true;
    videos = null;
    cursor = null;
    problem = null;
    editing = null;
    contentIpc
      .videos(k, null)
      .then((p) => {
        if (!alive) return;
        videos = p.videos;
        cursor = p.cursor;
        total = p.total;
        picked = kept(picked, p.videos.map((v) => v.id));
      })
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });

  function loadMore() {
    if (!cursor || more) return;
    more = true;
    contentIpc
      .videos(kindOf(kind), cursor)
      .then((p) => {
        videos = appended(videos ?? [], p.videos, (v) => v.id);
        cursor = p.cursor;
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (more = false));
  }

  function patch(id: string, change: Partial<ManagedVideo>) {
    videos = (videos ?? []).map((v) => (v.id === id ? { ...v, ...change } : v));
  }

  /** Runs one row's write, the row greyed out meanwhile. */
  async function busy(id: string, run: () => Promise<void>) {
    if (working.has(id)) return;
    working = new Set([...working, id]);
    try {
      await run();
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      working = new Set([...working].filter((x) => x !== id));
    }
  }

  function saveTitle() {
    const e = editing;
    if (!e) return;
    const v = videos?.find((x) => x.id === e.id);
    if (!v || e.text.trim() === v.title) {
      editing = null;
      return;
    }
    void busy(e.id, async () => {
      const title = await contentIpc.renameVideo(e.id, e.text);
      patch(e.id, { title });
      editing = null;
      toast.success('Renamed.');
    });
  }

  function publish(v: ManagedVideo, on: boolean) {
    void busy(v.id, async () => {
      const now = await contentIpc.publishVideo(v.id, on);
      patch(v.id, { public: now });
      toast.success(now ? 'Published.' : 'Unpublished. Only you can see it now.');
    });
  }

  /** Publish, Unpublish or Delete the ticked rows. */
  async function many(what: 'publish' | 'unpublish' | 'delete') {
    const ids = [...picked];
    if (!ids.length || bulk) return;
    bulk = true;
    try {
      const r = what === 'delete' ? await contentIpc.deleteVideos(ids) : await contentIpc.publishVideos(ids, what === 'publish');
      const done = new Set(r.gone);
      if (what === 'delete') {
        videos = (videos ?? []).filter((v) => !done.has(v.id));
        if (total !== null) total = Math.max(0, total - done.size);
      } else videos = (videos ?? []).map((v) => (done.has(v.id) ? { ...v, public: what === 'publish' } : v));
      picked = kept(picked, (videos ?? []).map((v) => v.id).filter((id) => !done.has(id)));
      const n = count(done.size, 'video', 'videos');
      if (done.size) toast.success(what === 'delete' ? `${n} deleted.` : what === 'publish' ? `${n} published.` : `${n} unpublished.`);
      if (r.error) toast.error(r.error);
      else if (done.size < ids.length) toast.error('Twitch kept some of them. Try again.');
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      bulk = false;
    }
  }

  function remove(v: ManagedVideo) {
    void busy(v.id, async () => {
      const r = await contentIpc.deleteVideos([v.id]);
      if (r.gone.includes(v.id)) {
        videos = (videos ?? []).filter((x) => x.id !== v.id);
        if (total !== null) total = Math.max(0, total - 1);
        toast.success('Video deleted.');
      } else toast.error(r.error ?? 'Twitch kept it. Try again.');
    });
  }

  function menu(e: MouseEvent, v: ManagedVideo) {
    const ready = v.state === 'ready';
    openMenuAt(e, [
      { label: 'Play', icon: 'play', disabled: v.state === 'processing', run: () => (playing = { kind: 'vod', id: v.id }) },
      { label: 'Rename', icon: 'pencil', run: () => (editing = { id: v.id, text: v.title }) },
      v.public
        ? { label: 'Unpublish', icon: 'eyeOff', disabled: !ready, note: 'Only you can see it then.', run: () => publish(v, false) }
        : { label: 'Publish', icon: 'eye', disabled: !ready, note: 'Anyone can watch it then.', run: () => publish(v, true) },
      { label: 'Copy link', icon: 'link', run: () => void copy(videoLink(v.id), 'Link') },
      'separator',
      { label: 'Delete video', icon: 'trash', danger: true, disabled: v.state === 'recording', note: 'Gone for good.', confirm: 'Click to delete', run: () => remove(v) },
    ]);
  }

  function focus(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

{#snippet thumb(v: ManagedVideo)}
  <span class="thumb">
    {#if v.thumbnail}<img src={v.thumbnail} alt="" loading="lazy" />{/if}
    <span class="len">{length(v.length)}</span>
  </span>
{/snippet}

<Card title="Videos" flush>
  <div class="bar">
    <Segmented label="Kind" options={VIDEO_KINDS} value={kind} onchange={(v) => (kind = v)} />
    <span class="end">
      {#if total}<span class="total">{formatNumber(total)}</span>{/if}
      <IconButton icon="refresh" label="Reload" size={14} onclick={() => reload++} />
    </span>
  </div>
  {#if problem}
    <EmptyState icon="alert" text={problem}><Button small onclick={() => reload++}>Try again</Button></EmptyState>
  {:else if !videos}
    <Loading rows={5} label="Loading" />
  {:else if !videos.length}
    <EmptyState icon="film" text={kind === 'all' ? 'No videos yet.' : 'None of these yet.'} />
  {:else}
    <div class="head">
      <label class="pick">
        <input type="checkbox" checked={all} indeterminate={picked.size > 0 && !all} onchange={() => (picked = all ? new Set() : new Set(videos?.map((v) => v.id)))} />
        <span class="what">{picked.size ? `${formatNumber(picked.size)} picked` : 'Pick all'}</span>
      </label>
      {#if picked.size}
        <span class="end">
          <IconButton icon="eye" label="Publish" hint="Anyone can watch them then." size={14} disabled={bulk} onclick={() => void many('publish')} />
          <IconButton icon="eyeOff" label="Unpublish" hint="Only you can see them then." size={14} disabled={bulk} onclick={() => void many('unpublish')} />
          <ConfirmButton
            label="Delete"
            confirm="Click to delete {count(picked.size, 'video', 'videos')}"
            icon="trash"
            disabled={bulk}
            onconfirm={() => void many('delete')}
          />
          <IconButton icon="x" label="Clear selection" size={14} onclick={() => (picked = new Set())} />
        </span>
      {/if}
    </div>
    <ul>
      {#each videos as v (v.id)}
        {@const tag = videoTag(v)}
        <li class:busy={working.has(v.id)} class:on={picked.has(v.id)}>
          <input type="checkbox" aria-label="Pick {v.title || 'video'}" checked={picked.has(v.id)} onchange={() => (picked = toggled(picked, v.id))} />
          {#if editing?.id === v.id}
            {@render thumb(v)}
            <form
              class="rename"
              onsubmit={(e) => {
                e.preventDefault();
                saveTitle();
              }}
            >
              <input
                class="field"
                aria-label="Title"
                maxlength={TITLE_MAX}
                bind:value={editing.text}
                use:focus
                onkeydown={(e) => {
                  if (e.key === 'Escape') {
                    e.preventDefault();
                    e.stopPropagation();
                    editing = null;
                  }
                }}
              />
              <Button small variant="primary" type="submit" disabled={!editing.text.trim() || working.has(v.id)}>Save</Button>
              <Button small variant="ghost" onclick={() => (editing = null)}>Cancel</Button>
            </form>
          {:else}
            <button class="item" disabled={v.state === 'processing'} onclick={() => (playing = { kind: 'vod', id: v.id })}>
              {@render thumb(v)}
              <span class="text">
                <span class="title">
                  <b>{v.title || 'Untitled'}</b>
                  {#if tag}<span class="tag" class:live={v.state === 'recording'}>{tag}</span>{/if}
                </span>
                <span class="line">{videoLine(v)}</span>
              </span>
            </button>
            <IconButton icon="more" label="More for {v.title || 'video'}" size={14} onclick={(e) => menu(e, v)} />
          {/if}
        </li>
      {/each}
    </ul>
    {#if cursor}
      <div class="more"><Button small variant="link" disabled={more} onclick={loadMore}>{more ? 'Loading…' : 'Show more'}</Button></div>
    {/if}
  {/if}
</Card>

{#if playing}<ContentPlayer what={playing} account={sm.manager?.accountId ?? null} onclose={() => (playing = null)} />{/if}

<style>
  /* A narrow panel scrolls a long picker instead of cutting it off. */
  .bar :global(.seg) {
    max-width: 100%;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
  }
  .what {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .end {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }
  .total {
    color: var(--dim);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  ul {
    margin: 0;
    padding: 4px 4px 6px;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: var(--radius);
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 34px;
    padding: 2px 10px;
  }
  .pick {
    min-width: 0;
    white-space: nowrap;
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: var(--dim);
    font-size: 0.78rem;
  }
  input[type='checkbox'] {
    flex: none;
    width: 14px;
    height: 14px;
    margin: 0;
    accent-color: var(--accent);
  }
  li.on,
  li:hover {
    background: var(--row-hover);
  }
  li.busy {
    opacity: 0.55;
    pointer-events: none;
  }
  .item {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    text-align: left;
    cursor: pointer;
  }
  .item:disabled {
    cursor: default;
  }
  .thumb {
    position: relative;
    flex: none;
    width: 96px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--elevated);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .len {
    position: absolute;
    right: 3px;
    bottom: 3px;
    padding: 0 4px;
    border-radius: var(--radius-sm);
    background: var(--scrim-strong);
    color: var(--on-video);
    font-size: 0.68rem;
    font-variant-numeric: tabular-nums;
  }
  .len:empty {
    display: none;
  }
  .text {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .title b,
  .line {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .title b {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .line {
    color: var(--dim);
    font-size: 0.76rem;
    font-variant-numeric: tabular-nums;
  }
  .tag {
    flex: none;
    padding: 0 5px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    color: var(--dim);
    font-size: 0.68rem;
    line-height: 1.4;
  }
  .tag.live {
    border-color: transparent;
    background: var(--live);
    color: var(--on-solid);
    font-weight: 700;
  }
  .rename {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .rename .field {
    flex: 1;
    min-width: 0;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 4px 0 10px;
  }
</style>
