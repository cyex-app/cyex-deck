<!-- V109-C03 Clips manager for one signed-in account: clips of its channel (by anyone) or
     clips it made (anywhere), by time and order, with a search and paging. A row plays in
     the app's clip player; ⋯ saves it as MP4 (V108-N02), copies the link, opens Twitch's
     editor for clips you made (V103-09) or deletes it (second click). Tick several to
     delete them at once. Self-contained: the host (the creator dashboard) gives the account. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Button from '../../components/Button.svelte';
  import ConfirmButton from '../../components/ConfirmButton.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Clip } from '../../ipc/bindings/Clip';
  import type { ClipOrder } from '../../ipc/bindings/ClipOrder';
  import type { ClipPeriod } from '../../ipc/bindings/ClipPeriod';
  import type { ClipsOf } from '../../ipc/bindings/ClipsOf';
  import { clipsIpc } from '../../ipc/clips';
  import { smIpc } from '../../ipc/streammanager';
  import { accounts } from '../../lib/accounts.svelte';
  import { copy } from '../../lib/clipboard';
  import { errorText } from '../../lib/errors';
  import { formatDate, formatNumber } from '../../lib/format';
  import { orToast } from '../../lib/orToast';
  import { PagedList } from '../../lib/paged.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { clipSaves } from '../player/clipSave.svelte';
  import ContentPlayer from '../streammanager/ContentPlayer.svelte';
  import { bulkLines, CLIP_PERIODS, clipLink, count, length, type Playing } from '../streammanager/content';
  import LoadMore from '../streammanager/LoadMore.svelte';
  import PickBar from '../streammanager/PickBar.svelte';
  import PickBox from '../streammanager/PickBox.svelte';
  import SmProblem from '../streammanager/SmProblem.svelte';
  import { emptyLine, madeBy, ORDERS, WHOSE } from './clips';

  interface Props {
    /** The signed-in account's user id; its channel is its own. */
    account: string;
    /** Which list opens first. */
    initial?: ClipsOf;
  }

  let { account, initial = 'channel' }: Props = $props();

  let of = $state<ClipsOf>(untrack(() => initial));
  let period = $state<ClipPeriod>('week');
  let order = $state<ClipOrder>('newest');
  let search = $state('');
  /** The search sent, a moment after typing stops. */
  let sent = $state('');
  let busy = $state(false);
  let playing = $state<Playing | null>(null);

  const me = $derived(accounts.list.find((a) => a.userId === account));
  const names = $derived(me ? [me.login, me.displayName] : []);

  // Each page reads the filters as they are; a filter change reloads and drops pages for the old one.
  const list = new PagedList<Clip>({
    key: (c) => c.slug,
    fetch: (cursor) =>
      clipsIpc.list(account, { of, period, order, search: sent, cursor: cursor ?? undefined }).then((p) => ({ items: p.clips, next: p.cursor })),
  });
  const clips = $derived(list.items);
  const picked = $derived(list.picked);

  $effect(() => {
    const text = search.trim();
    const t = setTimeout(() => (sent = text), 350);
    return () => clearTimeout(t);
  });

  $effect(() => {
    void [account, of, period, order, sent];
    untrack(() => void list.reload({ clear: true }));
  });

  const day = (iso: string) => {
    const t = Date.parse(iso);
    return Number.isNaN(t) ? '' : formatDate(new Date(t));
  };
  const views = (n: number) => `${formatNumber(n)} ${n === 1 ? 'view' : 'views'}`;
  const who = (c: Clip) => (of === 'mine' ? c.channelName && `in ${c.channelName}` : c.clippedBy && `by ${c.clippedBy}`);

  async function remove(slugs: string[]) {
    if (!slugs.length || busy) return;
    busy = true;
    try {
      const r = await clipsIpc.remove(account, slugs);
      list.remove(r.done);
      const say = bulkLines(r, slugs.length, (n) => (slugs.length === 1 ? 'Clip deleted.' : `${count(n, 'clip', 'clips')} deleted.`));
      if (say.ok) toast.success(say.ok);
      if (say.info) toast.info(say.info);
      if (say.bad) toast.error(say.bad);
    } catch (e) {
      toast.error(errorText(e));
    } finally {
      busy = false;
    }
  }

  async function edit(c: Clip) {
    try {
      const opened = await clipsIpc.edit(account, c.slug);
      if (!opened) toast.info('Mock run: Twitch’s editor stays closed.');
    } catch (e) {
      toast.error(errorText(e, 'open the editor'));
    }
  }

  function menu(e: MouseEvent, c: Clip) {
    e.preventDefault();
    const saving = clipSaves.has(c.slug);
    openMenuAt(e, [
      { label: 'Play', icon: 'play', run: () => (playing = { kind: 'clip', slug: c.slug }) },
      saving
        ? { label: 'Stop saving', icon: 'x', hint: clipSaves.text(c.slug), run: () => clipSaves.cancel(c.slug) }
        : { label: 'Save as MP4', icon: 'download', run: () => void clipSaves.save(c.slug) },
      { label: 'Copy link', icon: 'link', run: () => void copy(clipLink(c), 'Link') },
      ...(madeBy(c, of, names)
        ? [{ label: 'Edit on Twitch', icon: 'pencil' as const, note: 'Title, start and end, in Twitch’s editor', run: () => void edit(c) }]
        : []),
      'separator',
      { label: 'Delete clip', icon: 'trash', danger: true, note: 'Gone for good.', confirm: 'Click to delete', run: () => void remove([c.slug]) },
    ]);
  }
</script>

<section class="clips" aria-label="Clips">
  <div class="bar">
    <SearchField small bind:value={search} placeholder={of === 'mine' ? 'Search titles or channels' : 'Search titles or clippers'} />
    <div class="filters">
      <Segmented label="Whose clips" options={WHOSE} value={of} onchange={(v) => (of = v)} />
      <Segmented label="Time" options={CLIP_PERIODS} value={period} onchange={(v) => (period = v)} />
      <Segmented label="Order" options={ORDERS} value={order} onchange={(v) => (order = v)} />
      <span class="end"><IconButton icon="refresh" label="Refresh" size={14} disabled={list.loading} onclick={() => void list.reload()} /></span>
    </div>
  </div>
  {#if list.failed && !clips.length}
    <SmProblem text={list.failed} onretry={() => void list.reload()} />
  {:else if list.loading && !clips.length}
    <Loading rows={6} label="Loading clips" />
  {:else if !clips.length}
    <EmptyState icon="clapper" text={emptyLine(of, !!sent)} />
  {:else}
    <PickBar picked={picked.size} total={clips.length} onall={list.pickAll}>
      {#if busy && picked.size > 1}
        <Button small variant="ghost" onclick={() => orToast(smIpc.bulkStop(), 'stop')}>Stop</Button>
      {:else if picked.size}
        <ConfirmButton
          label="Delete"
          confirm="Click to delete {count(picked.size, 'clip', 'clips')}"
          icon="trash"
          disabled={busy}
          onconfirm={() => void remove([...picked])}
        />
        <IconButton icon="x" label="Clear selection" size={14} onclick={() => list.pickAll(false)} />
      {/if}
    </PickBar>
    <ul>
      {#each clips as c (c.slug)}
        <li class:on={picked.has(c.slug)} oncontextmenu={(e) => menu(e, c)}>
          <PickBox label="Pick {c.title || 'clip'}" checked={picked.has(c.slug)} onchange={() => list.toggle(c.slug)} />
          <button class="item" onclick={() => (playing = { kind: 'clip', slug: c.slug })}>
            <span class="thumb">
              {#if c.thumbnail}<img src={c.thumbnail} alt="" loading="lazy" />{/if}
              <span class="len">{length(c.duration)}</span>
            </span>
            <span class="text">
              <b>{c.title || 'Untitled'}</b>
              <span class="meta">
                {#if who(c)}<span class="who">{who(c)}</span>{/if}
                <span class="data">{[day(c.createdAt), views(c.views)].filter(Boolean).join(' · ')}</span>
              </span>
            </span>
          </button>
          {#if clipSaves.has(c.slug)}<span class="saving">{clipSaves.text(c.slug)}</span>{/if}
          <IconButton icon="more" label="More for {c.title || 'clip'}" size={14} onclick={(e) => menu(e, c)} />
        </li>
      {/each}
    </ul>
    {#if list.failed}<p class="bad">{list.failed}</p>{/if}
    {#if list.next}<LoadMore busy={list.more} onclick={() => void list.loadMore()} />{/if}
  {/if}
</section>

{#if playing}<ContentPlayer what={playing} {account} onclose={() => (playing = null)} />{/if}

<style>
  .clips {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    container-type: inline-size;
    overflow: auto;
  }
  .bar {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .end {
    margin-left: auto;
  }
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
    padding: 6px 10px;
    border-radius: var(--radius);
  }
  li:hover {
    background: var(--row-hover);
    box-shadow: inset 2px 0 0 var(--border-strong);
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
    width: 96px;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--elevated-2);
  }
  @container (max-width: 360px) {
    .thumb {
      width: 64px;
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
    font-family: var(--font-mono);
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
    font-size: 0.85rem;
    font-weight: 600;
  }
  .meta {
    display: flex;
    flex-wrap: wrap;
    column-gap: 8px;
    color: var(--dim);
    font-size: 0.75rem;
  }
  .who {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .data,
  .saving {
    font-family: var(--font-mono);
    font-size: 0.72rem;
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .saving {
    flex: none;
    color: var(--dim);
  }
</style>
