<!-- V107-R39 Video producer: your past broadcasts, highlights and uploads, unpublished ones
     too. A video plays in the app's VOD player; its ⋯ renames it (in the row), publishes or
     unpublishes it, copies the link or deletes it (a second click). -->
<script lang="ts">
  import { untrack } from 'svelte';

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
  import { smIpc } from '../../ipc/streammanager';
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { PagedList } from '../../lib/paged.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ContentPlayer from './ContentPlayer.svelte';
  import { limits } from './limits.svelte';
  import LoadMore from './LoadMore.svelte';
  import PickBar from './PickBar.svelte';
  import PickBox from './PickBox.svelte';
  import SmBar from './SmBar.svelte';
  import SmProblem from './SmProblem.svelte';
  import { bulkLines, count, type KindPick, kindOf, length, type Playing, VIDEO_KINDS, videoLine, videoLink, videoTag } from './content';
  import { sm } from './streammanager.svelte';

  limits.load();

  let kind = $state<KindPick>('all');
  let playing = $state<Playing | null>(null);
  /** The row being renamed and its text. */
  let editing = $state<{ id: string; text: string } | null>(null);
  /** Rows with a write on the way. */
  let working = $state<Set<string>>(new Set());
  let bulk = $state(false);

  // BUG-02 Each page reads the kind as it is; a kind change reloads (and resets the total),
  // so a page for the old one is dropped.
  const list = new PagedList<ManagedVideo>({
    key: (v) => v.id,
    fetch: (cursor) => contentIpc.videos(kindOf(kind), cursor).then((p) => ({ items: p.videos, next: p.cursor, total: p.total })),
  });
  const videos = $derived(list.items);
  const picked = $derived(list.picked);
  const total = $derived(list.total);

  $effect(() => {
    void kind;
    untrack(() => {
      editing = null;
      void list.reload({ clear: true });
    });
  });

  function patch(id: string, change: Partial<ManagedVideo>) {
    list.patch(id, (v) => ({ ...v, ...change }));
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
    const v = videos.find((x) => x.id === e.id);
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

  const DID = {
    publish: (n: number) => `${count(n, 'video', 'videos')} published.`,
    unpublish: (n: number) => `${count(n, 'video', 'videos')} unpublished.`,
    delete: (n: number) => `${count(n, 'video', 'videos')} deleted.`,
  };

  /** Publish, Unpublish or Delete the ticked rows. */
  async function many(what: 'publish' | 'unpublish' | 'delete') {
    const ids = [...picked];
    if (!ids.length || bulk) return;
    bulk = true;
    try {
      const r = what === 'delete' ? await contentIpc.deleteVideos(ids) : await contentIpc.publishVideos(ids, what === 'publish');
      if (what === 'delete') list.remove(r.done);
      else for (const id of r.done) patch(id, { public: what === 'publish' });
      const say = bulkLines(r, ids.length, DID[what]);
      if (say.ok) toast.success(say.ok);
      if (say.info) toast.info(say.info);
      if (say.bad) toast.error(say.bad);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      bulk = false;
    }
  }

  function remove(v: ManagedVideo) {
    void busy(v.id, async () => {
      const r = await contentIpc.deleteVideos([v.id]);
      if (r.done.includes(v.id)) {
        list.remove([v.id]);
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
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={list.loading} onclick={() => void list.reload()} />
  {/snippet}
  <SmBar>
    <div class="filters">
      <Segmented label="Kind" options={VIDEO_KINDS} value={kind} onchange={(v) => (kind = v)} />
      {#if total}<span class="count">{count(total, 'video', 'videos')}</span>{/if}
    </div>
  </SmBar>
  {#if list.failed && !videos.length}
    <SmProblem text={list.failed} onretry={() => void list.reload()} />
  {:else if list.loading && !videos.length}
    <Loading rows={5} label="Loading videos" />
  {:else if !videos.length}
    <EmptyState icon="film" text={kind === 'all' ? 'No videos yet.' : 'None of these yet.'} />
  {:else}
    <PickBar picked={picked.size} total={videos.length} onall={list.pickAll}>
      {#if bulk && picked.size > 1}
        <Button small variant="ghost" onclick={() => void smIpc.bulkStop()}>Stop</Button>
      {:else if picked.size}
        <IconButton icon="eye" label="Publish" hint="Anyone can watch them then." size={14} disabled={bulk} onclick={() => void many('publish')} />
        <IconButton icon="eyeOff" label="Unpublish" hint="Only you can see them then." size={14} disabled={bulk} onclick={() => void many('unpublish')} />
        <ConfirmButton
          label="Delete"
          confirm="Click again to delete {count(picked.size, 'video', 'videos')}"
          icon="trash"
          disabled={bulk}
          onconfirm={() => void many('delete')}
        />
        <IconButton icon="x" label="Clear selection" size={14} onclick={() => list.pickAll(false)} />
      {/if}
    </PickBar>
    <ul>
      {#each videos as v (v.id)}
        {@const tag = videoTag(v)}
        <li class:busy={working.has(v.id)} class:on={picked.has(v.id)}>
          <PickBox label="Pick {v.title || 'video'}" checked={picked.has(v.id)} onchange={() => list.toggle(v.id)} />
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
                maxlength={limits.v?.videoTitleMax}
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
                <b class="title">{v.title || 'Untitled'}</b>
                <span class="meta">{#if tag}<span class="tag" class:live={v.state === 'recording'}>{tag}</span>{/if}{videoLine(v)}</span>
              </span>
            </button>
            <IconButton icon="more" label="More for {v.title || 'video'}" size={14} onclick={(e) => menu(e, v)} />
          {/if}
        </li>
      {/each}
    </ul>
    {#if list.failed}<p class="bad">{list.failed}</p>{/if}
    {#if list.next}<LoadMore busy={list.more} onclick={() => void list.loadMore()} />{/if}
  {/if}
</Card>

{#if playing}<ContentPlayer what={playing} account={sm.manager?.accountId ?? null} onclose={() => (playing = null)} />{/if}

<style>
  .bad {
    margin: 0;
    padding: 4px 12px;
    color: var(--red);
    font-size: 0.82rem;
  }
  ul {
    margin: 0;
    padding: 4px;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: var(--radius);
  }
  li:hover {
    background: var(--row-hover);
  }
  li.on {
    background: var(--surface-hover);
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
    width: 80px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--elevated-2);
  }
  /* A narrow panel gives the words the room. */
  @container (max-width: 320px) {
    .thumb {
      width: 56px;
    }
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
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Two lines at most, so a narrow panel keeps the views. */
  .meta {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .title {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .meta {
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .tag {
    margin-right: 6px;
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
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .rename .field {
    flex: 1;
    min-width: 0;
  }
</style>
