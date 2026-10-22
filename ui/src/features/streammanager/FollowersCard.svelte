<!-- V107-R40 Dashboard › Community › Followers list: your channel's followers, newest
     first, with how long ago they followed (hover: the date and time). Search and a name sort work on
     what is loaded; "Load more" adds the next page. A click opens the usercard. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import EmptyState from '../../components/EmptyState.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import Loading from '../../components/Loading.svelte';
  import Card from '../../components/PanelCard.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import SortControl from '../../components/SortControl.svelte';
  import type { CmFollower } from '../../ipc/bindings/CmFollower';
  import { chatIpc } from '../../ipc/chat';
  import { cmIpc } from '../../ipc/community';
  import { errorText } from '../../lib/errors';
  import { formatNumber } from '../../lib/format';
  import { PagedList } from '../../lib/paged.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { FOLLOW_SORT, FOLLOW_SORTS, followsNewest, shownFollowers } from './community';
  import LoadMore from './LoadMore.svelte';
  import SmBar from './SmBar.svelte';
  import SmProblem from './SmProblem.svelte';
  import { sm } from './streammanager.svelte';
  import When from './When.svelte';

  const login = $derived(sm.manager?.login ?? '');

  let sort = $state.raw(FOLLOW_SORT);
  let query = $state('');
  const newest = $derived(followsNewest(sort));
  /** Only the newest request counts, so a slow old one can't overwrite a newer list. */
  const paged = new PagedList<CmFollower>({ key: (f) => f.id, fetch: (cursor) => cmIpc.followers(cursor, newest) });
  const list = $derived(paged.items);
  const total = $derived(paged.total ?? 0);
  const loading = $derived(paged.loading);
  const failed = $derived(paged.failed);
  const shown = $derived(shownFollowers(list, query, sort));

  // The first page, and again when the date order flips (Twitch orders it, not us).
  $effect(() => {
    void newest;
    untrack(() => void paged.reload());
  });

  function open(f: CmFollower) {
    chatIpc.openUsercard(login, f.login).catch((e) => toast.error(errorText(e)));
  }
</script>

<Card title="Followers" flush>
  {#snippet tools()}
    <IconButton icon="refresh" label="Refresh" size={14} disabled={loading} onclick={() => void paged.reload()} />
  {/snippet}
  <SmBar>
    <SearchField small bind:value={query} placeholder="Search followers" />
    <div class="filters">
      <SortControl keys={FOLLOW_SORTS} {sort} label="Sort followers" onchange={(s) => (sort = s)} />
      {#if !failed && !loading}<span class="count">{formatNumber(total)} {total === 1 ? 'follower' : 'followers'}</span>{/if}
    </div>
  </SmBar>
  {#if loading && !list.length}
    <Loading rows={6} label="Loading followers" />
  {:else if failed && !list.length}
    <SmProblem text={failed} onretry={() => void paged.reload()} />
  {:else if !list.length}
    <EmptyState icon="users" text="No followers yet." />
  {:else}
    <ul>
      {#each shown as f (f.id)}
        <li>
          <button class="pick" onclick={() => open(f)}>
            <Avatar src={f.avatar} name={f.display} size={28} />
            <span class="who">
              <span class="name">{f.display}</span>
              {#if f.display.toLowerCase() !== f.login}<span class="meta">{f.login}</span>{/if}
            </span>
            {#if f.followedAt}<When at={f.followedAt} />{/if}
          </button>
        </li>
      {/each}
    </ul>
    {#if query && !shown.length}<p class="note pad">No one loaded matches.</p>{/if}
    {#if failed}<p class="bad pad">{failed}</p>{/if}
    {#if paged.next}
      <LoadMore busy={paged.more} note={query ? `Search covers the ${formatNumber(list.length)} loaded.` : ''} onclick={() => void paged.loadMore()} />
    {/if}
  {/if}
</Card>

<style>
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
    font-size: 0.85rem;
  }
  .meta {
    color: var(--dim);
    font-size: 0.75rem;
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
