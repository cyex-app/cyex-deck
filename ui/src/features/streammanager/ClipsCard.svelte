<!-- V107-R39 Clips manager: clips of your channel or clips you made, by period and order,
     with a search. A clip plays in the app's clip player; its ⋯ copies the link or deletes
     it (a second click). Tick several to delete them at once. -->
<script lang="ts">
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
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import ContentPlayer from './ContentPlayer.svelte';
  import { appended, CLIP_ORDERS, CLIP_PERIODS, CLIPS_OF, clipLine, clipLink, count, kept, length, type Playing, toggled } from './content';
  import { sm } from './streammanager.svelte';

  let of = $state<ClipsOf>('channel');
  let period = $state<ClipPeriod>('week');
  let order = $state<ClipOrder>('views');
  let search = $state('');
  /** The search sent, a moment after typing stops. */
  let sent = $state('');
  let clips = $state<Clip[] | null>(null);
  let cursor = $state<string | null>(null);
  let problem = $state<string | null>(null);
  let more = $state(false);
  let busy = $state(false);
  let picked = $state<Set<string>>(new Set());
  let playing = $state<Playing | null>(null);
  let reload = $state(0);

  $effect(() => {
    const text = search.trim();
    const t = setTimeout(() => (sent = text), 350);
    return () => clearTimeout(t);
  });

  $effect(() => {
    const filter = { of, period, order, search: sent };
    void reload;
    let alive = true;
    clips = null;
    cursor = null;
    problem = null;
    contentIpc
      .clips(filter)
      .then((p) => {
        if (!alive) return;
        clips = p.clips;
        cursor = p.cursor;
        picked = kept(picked, p.clips.map((c) => c.slug));
      })
      .catch((e) => alive && (problem = errorText(e)));
    return () => (alive = false);
  });

  function loadMore() {
    if (!cursor || more) return;
    more = true;
    contentIpc
      .clips({ of, period, order, search: sent, cursor })
      .then((p) => {
        clips = appended(clips ?? [], p.clips, (c) => c.slug);
        cursor = p.cursor;
      })
      .catch((e) => toast.error(errorText(e)))
      .finally(() => (more = false));
  }

  async function remove(slugs: string[]) {
    if (!slugs.length || busy) return;
    busy = true;
    try {
      const r = await contentIpc.deleteClips(slugs);
      const gone = new Set(r.gone);
      clips = (clips ?? []).filter((c) => !gone.has(c.slug));
      picked = kept(picked, clips.map((c) => c.slug));
      if (r.gone.length) toast.success(`${count(r.gone.length, 'clip', 'clips')} deleted.`);
      if (r.error) toast.error(r.error);
      else if (r.gone.length < slugs.length) toast.error('Twitch kept some of them. Try again.');
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

  const all = $derived(!!clips?.length && picked.size === clips.length);
</script>

<Card title="Clips" flush>
  <div class="bar">
    <Segmented label="Whose clips" options={CLIPS_OF} value={of} onchange={(v) => (of = v)} />
    <span class="end"><IconButton icon="refresh" label="Reload" size={14} onclick={() => reload++} /></span>
    <Segmented label="Time" options={CLIP_PERIODS} value={period} onchange={(v) => (period = v)} />
    <Segmented label="Order" options={CLIP_ORDERS} value={order} onchange={(v) => (order = v)} />
    <SearchField small bind:value={search} placeholder={of === 'mine' ? 'Search titles or channels' : 'Search titles or clippers'} />
  </div>
  {#if problem}
    <EmptyState icon="alert" text={problem}><Button small onclick={() => reload++}>Try again</Button></EmptyState>
  {:else if !clips}
    <Loading rows={5} label="Loading" />
  {:else if !clips.length}
    <EmptyState icon="clapper" text={sent ? 'No clips match that.' : 'No clips in this time.'} />
  {:else}
    <div class="head">
      <label class="pick">
        <input type="checkbox" checked={all} indeterminate={picked.size > 0 && !all} onchange={() => (picked = all ? new Set() : new Set(clips?.map((c) => c.slug)))} />
        <span class="what">{picked.size ? `${formatNumber(picked.size)} picked` : 'Pick all'}</span>
      </label>
      {#if picked.size}
        <span class="end">
          <ConfirmButton
            label="Delete"
            confirm="Click to delete {count(picked.size, 'clip', 'clips')}"
            icon="trash"
            disabled={busy}
            onconfirm={() => void remove([...picked])}
          />
          <IconButton icon="x" label="Clear selection" size={14} onclick={() => (picked = new Set())} />
        </span>
      {/if}
    </div>
    <ul>
      {#each clips as c (c.slug)}
        <li class:on={picked.has(c.slug)}>
          <input type="checkbox" aria-label="Pick {c.title || 'clip'}" checked={picked.has(c.slug)} onchange={() => (picked = toggled(picked, c.slug))} />
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
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 34px;
    padding: 2px 10px;
  }
  .what {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .end {
    flex: none;
    display: flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
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
  ul {
    margin: 0;
    padding: 2px 4px 6px;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    border-radius: var(--radius);
  }
  li:hover,
  li.on {
    background: var(--row-hover);
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
    border-radius: 3px;
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
  .text b,
  .text span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .text b {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .text span {
    color: var(--dim);
    font-size: 0.76rem;
    font-variant-numeric: tabular-nums;
  }
  .more {
    display: flex;
    justify-content: center;
    padding: 4px 0 10px;
  }
</style>
