<!-- V107-R39 Clips manager: clips of your channel or clips you made, by period and order,
     with a search. A clip plays in the app's clip player; its ⋯ copies the link or deletes
     it (a second click). Tick several to delete them at once. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Clip } from '../../ipc/bindings/Clip';
  import type { ClipOrder } from '../../ipc/bindings/ClipOrder';
  import type { ClipPeriod } from '../../ipc/bindings/ClipPeriod';
  import type { ClipsOf } from '../../ipc/bindings/ClipsOf';
  import { contentIpc } from '../../ipc/content';
  import { smIpc } from '../../ipc/streammanager';
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { PagedList } from '../../lib/paged.svelte';
  import { toast } from '../../lib/toast.svelte';
  import ContentPlayer from './ContentPlayer.svelte';
  import LoadMore from './LoadMore.svelte';
  import PickBar from './PickBar.svelte';
  import PickBox from './PickBox.svelte';
  import SmBar from './SmBar.svelte';
  import SmProblem from './SmProblem.svelte';
  import { bulkLines, CLIP_ORDERS, CLIP_PERIODS, CLIPS_OF, clipLine, clipLink, count, length, type Playing } from './content';
  import { sm } from './streammanager.svelte';

  let of = $state<ClipsOf>('channel');
  let period = $state<ClipPeriod>('week');
  let order = $state<ClipOrder>('views');
  let search = $state('');
  /** The search sent, a moment after typing stops. */
  let sent = $state('');
  let busy = $state(false);
  let playing = $state<Playing | null>(null);

  // BUG-02 Each page reads the filters as they are; a filter change reloads, so a page for
  // the old one is dropped.
  const list = new PagedList<Clip>({
    key: (c) => c.slug,
    fetch: (cursor) => contentIpc.clips({ of, period, order, search: sent, cursor: cursor ?? undefined }).then((p) => ({ items: p.clips, next: p.cursor })),
  });
  const clips = $derived(list.items);
  const picked = $derived(list.picked);

  $effect(() => {
    const text = search.trim();
    const t = setTimeout(() => (sent = text), 350);
    return () => clearTimeout(t);
  });

  $effect(() => {
    void [of, period, order, sent];
    untrack(() => void list.reload({ clear: true }));
  });

  async function remove(slugs: string[]) {
    if (!slugs.length || busy) return;
    busy = true;
    try {
      const r = await contentIpc.deleteClips(slugs);
      list.remove(r.done);
      const say = bulkLines(r, slugs.length, (n) => `${count(n, 'clip', 'clips')} deleted.`);
      if (say.ok) toast.success(say.ok);
      if (say.info) toast.info(say.info);
      if (say.bad) toast.error(say.bad);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  function menu(e: MouseEvent, c: Clip) {
    openMenuAt(e, [
      { label: 'Play', icon: 'play', run: () => (playing = { kind: 'clip', slug: c.slug }) },
      { label: 'Copy link', icon: 'link', run: () => void copy(clipLink(c), 'Link') },
      'separator',
      { label: 'Delete clip', icon: 'trash', danger: true, note: 'Gone for good.', confirm: 'Click to delete', run: () => void remove([c.slug]) },
    ]);
  }
</script>

<Card title="Clips" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={list.loading} onclick={() => void list.reload()} />
  {/snippet}
  <SmBar>
    <SearchField small bind:value={search} placeholder={of === 'mine' ? 'Search titles or channels' : 'Search titles or clippers'} />
    <div class="filters">
      <Segmented label="Whose clips" options={CLIPS_OF} value={of} onchange={(v) => (of = v)} />
      <Segmented label="Time" options={CLIP_PERIODS} value={period} onchange={(v) => (period = v)} />
      <Segmented label="Order" options={CLIP_ORDERS} value={order} onchange={(v) => (order = v)} />
    </div>
  </SmBar>
  {#if list.failed && !clips.length}
    <SmProblem text={list.failed} onretry={() => void list.reload()} />
  {:else if list.loading && !clips.length}
    <Loading rows={5} label="Loading clips" />
  {:else if !clips.length}
    <EmptyState icon="clapper" text={sent ? 'No clips match that.' : 'No clips in this time.'} />
  {:else}
    <PickBar picked={picked.size} total={clips.length} onall={list.pickAll}>
      {#if busy && picked.size > 1}
        <Button small variant="ghost" onclick={() => void smIpc.bulkStop()}>Stop</Button>
      {:else if picked.size}
        <ConfirmButton
          label="Delete"
          confirm="Click again to delete {count(picked.size, 'clip', 'clips')}"
          icon="trash"
          disabled={busy}
          onconfirm={() => void remove([...picked])}
        />
        <IconButton icon="x" label="Clear selection" size={14} onclick={() => list.pickAll(false)} />
      {/if}
    </PickBar>
    <ul>
      {#each clips as c (c.slug)}
        <li class:on={picked.has(c.slug)}>
          <PickBox label="Pick {c.title || 'clip'}" checked={picked.has(c.slug)} onchange={() => list.toggle(c.slug)} />
          <button class="item" onclick={() => (playing = { kind: 'clip', slug: c.slug })}>
            <span class="thumb">
              {#if c.thumbnail}<img src={c.thumbnail} alt="" loading="lazy" />{/if}
              <span class="len">{length(c.duration)}</span>
            </span>
            <span class="text">
              <b>{c.title || 'Untitled'}</b>
              <span>{clipLine(c, of === 'mine')}</span>
            </span>
          </button>
          <IconButton icon="more" label="More for {c.title || 'clip'}" size={14} onclick={(e) => menu(e, c)} />
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
  .text b {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* Two lines at most, so a narrow panel keeps the views. */
  .text span {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .text b {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .text span {
    color: var(--dim);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
</style>
