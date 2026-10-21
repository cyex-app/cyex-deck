<!-- V107-R40 Dashboard › Community › Followers list: your channel's followers, newest
     first, with the day they followed (hover: how long ago). Search and a name sort work on
     what is loaded; "Load more" adds the next page. A click opens the usercard. -->
<script lang="ts">
  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import type { CmFollower } from '../../ipc/bindings/CmFollower';
  import { chatIpc } from '../../ipc/chat';
  import { cmIpc } from '../../ipc/community';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { toast } from '../../lib/toast.svelte';
  import { FOLLOW_SORTS, type FollowSort, merged, relative, shownFollowers, stamp } from './community';
  import { sm } from './streammanager.svelte';

  const login = $derived(sm.manager?.login ?? '');

  let sort = $state<FollowSort>('newest');
  let query = $state('');
  let list = $state<CmFollower[]>([]);
  let total = $state(0);
  let next = $state<string | null>(null);
  let loading = $state(true);
  let more = $state(false);
  let failed = $state('');
  /** Only the newest request counts, so a slow old one can't overwrite a newer list. */
  let seq = 0;

  const newest = $derived(sort !== 'oldest');
  const shown = $derived(shownFollowers(list, query, sort));

  async function load(from: string | null) {
    const mine = ++seq;
    if (from) more = true;
    else loading = true;
    try {
      const p = await cmIpc.followers(from, newest);
      if (mine !== seq) return;
      list = from ? merged(list, p.items) : p.items;
      total = p.total;
      next = p.next;
      failed = '';
    } catch (e) {
      if (mine !== seq) return;
      failed = errorText(e);
    } finally {
      if (mine === seq) {
        loading = false;
        more = false;
      }
    }
  }

  // The first page, and again when the date order flips (Twitch orders it, not us).
  $effect(() => {
    void newest;
    void load(null);
  });

  function open(f: CmFollower) {
    chatIpc.openUsercard(login, f.login).catch((e) => toast.error(errorText(e)));
  }
</script>

<Card title="Followers" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={() => void load(null)} />
  {/snippet}
  <div class="bar">
    <SearchField small bind:value={query} placeholder="Search followers" />
    <div class="row">
      <Segmented options={FOLLOW_SORTS} value={sort} label="Sort followers" onchange={(v) => (sort = v)} />
      {#if !failed && !loading}<span class="count">{formatNumber(total)} followers</span>{/if}
    </div>
  </div>
  {#if loading && !list.length}
    <Loading rows={6} label="Loading followers" />
  {:else if failed && !list.length}
    <div class="gap">
      <p class="bad">{failed}</p>
      <Button small onclick={() => void load(null)}>Retry</Button>
    </div>
  {:else if !list.length}
    <EmptyState icon="users" text="No followers yet." />
  {:else}
    <ul>
      {#each shown as f (f.id)}
        <li>
          <button class="pick" onclick={() => open(f)}>
            <Avatar src={f.avatar} name={f.name} size={28} />
            <span class="who">
              <span class="name">{f.name}</span>
              {#if f.name.toLowerCase() !== f.login}<span class="meta">{f.login}</span>{/if}
            </span>
            {#if f.followedAt}
              <time datetime={f.followedAt} use:tooltip={relative(f.followedAt)}>{stamp(f.followedAt)}</time>
            {/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if query && !shown.length}<p class="note pad">No one loaded matches.</p>{/if}
    {#if failed}<p class="bad pad">{failed}</p>{/if}
    {#if next}
      <div class="foot">
        {#if query}<span class="note">Search covers the {formatNumber(list.length)} loaded.</span>{/if}
        <Button small disabled={more} onclick={() => void load(next)}>{more ? 'Loading…' : 'Load more'}</Button>
      </div>
    {/if}
  {/if}
</Card>

<style>
  .bar {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }
  .count {
    color: var(--muted);
    font-size: 0.78rem;
    font-variant-numeric: tabular-nums;
  }
  ul {
    margin: 0;
    padding: 4px;
    list-style: none;
  }
  .pick {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 4px 6px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--text);
    text-align: left;
  }
  .pick:hover {
    background: var(--row-hover);
  }
  .who {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .name,
  .meta {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name {
    font-weight: 600;
    font-size: 0.86rem;
  }
  .meta {
    color: var(--dim);
    font-size: 0.75rem;
  }
  time {
    flex: none;
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .foot {
    display: flex;
    align-items: center;
    justify-content: center;
    flex-wrap: wrap;
    gap: 8px;
    padding: 8px;
  }
  .gap {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 8px;
    padding: 12px;
  }
  .bad {
    margin: 0;
    color: var(--red);
    font-size: 0.82rem;
  }
  .pad {
    padding: 4px 12px;
  }
</style>
