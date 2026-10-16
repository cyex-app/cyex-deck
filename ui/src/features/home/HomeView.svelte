<!-- Home (1.6): channels you follow, live ones first, as a grid or a list. Starred channels
     (H2) sit on top; the filter box and sorting are H1. -->
<script lang="ts">
  import { untrack } from 'svelte';

  import Avatar from '../../components/Avatar.svelte';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { type MenuEntry, openMenuAt } from '../../components/menu.svelte';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import type { GameRef } from '../../ipc/bindings/GameRef';
  import type { LiveSort } from '../../ipc/bindings/LiveSort';
  import type { OfflineSort } from '../../ipc/bindings/OfflineSort';
  import { accounts } from '../../lib/accounts.svelte';
  import { settings } from '../../lib/settings.svelte';
  import ChannelRow from '../browse/ChannelRow.svelte';
  import ChannelTile from '../browse/ChannelTile.svelte';
  import { sections } from '../browse/lists';
  import type { Nav } from '../browse/open.svelte';
  import Peek from '../browse/Peek.svelte';
  import PickedBar from '../browse/PickedBar.svelte';
  import StreamCard from '../browse/StreamCard.svelte';
  import { discover } from '../discover/discover.svelte';
  import { home } from './home.svelte';
  import Loading from '../../components/Loading.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { keepScroll } from '../../lib/keepScroll';

  interface Props {
    /** Opens a tab. */
    onopen: (tabId: string) => void;
    ondiscover: () => void;
    onsignin: () => void;
  }

  let { onopen, ondiscover, onsignin }: Props = $props();

  const nav: Nav = { show: (id) => onopen(id), account: () => home.account };

  let query = $state('');
  let offlineOpen = $state(true);

  const prefs = $derived(settings.v.browse);
  const grid = $derived(prefs.layout === 'grid');
  const lists = $derived(
    sections(home.follows ?? [], home.favorites, { live: prefs.liveSort, offline: prefs.offlineSort }, query),
  );
  const owner = $derived(accounts.list.find((a) => a.userId === home.account));
  const favoriteIds = $derived(prefs.favorites.map((c) => c.id).join());

  // Polls while home shows, and starts over for another account.
  $effect(() => {
    void home.account;
    return untrack(() => home.watch());
  });

  let firstFavorites = true;
  $effect(() => {
    void favoriteIds;
    if (firstFavorites) firstFavorites = false;
    else untrack(() => void home.reloadFavorites());
  });

  function category(game: GameRef) {
    discover.go({ kind: 'category', slug: game.slug, name: game.name });
    ondiscover();
  }

  function tag(t: string) {
    discover.showTag(t);
    if (discover.place.kind === 'category') discover.go({ kind: 'streams' });
    ondiscover();
  }

  function pickAccount(e: MouseEvent) {
    const items: MenuEntry[] = home.able.map((id) => {
      const a = accounts.list.find((x) => x.userId === id)!;
      return { label: a.displayName, checked: id === home.account, run: () => (home.picked = id) };
    });
    openMenuAt(e, items);
  }

  function sortMenu(e: MouseEvent) {
    const live: [LiveSort, string][] = [
      ['viewers', 'Most viewers'],
      ['uptime', 'Live the longest'],
      ['name', 'Name'],
    ];
    const offline: [OfflineSort, string][] = [
      ['lastLive', 'Live most recently'],
      ['name', 'Name'],
    ];
    openMenuAt(e, [
      {
        label: 'Live channels',
        items: live.map(([v, label]) => ({
          label,
          checked: prefs.liveSort === v,
          run: () => void settings.patch('browse', { liveSort: v }),
        })),
      },
      {
        label: 'Offline channels',
        items: offline.map(([v, label]) => ({
          label,
          checked: prefs.offlineSort === v,
          run: () => void settings.patch('browse', { offlineSort: v }),
        })),
      },
    ]);
  }

  const setLayout = (layout: 'grid' | 'list') => void settings.patch('browse', { layout });
  const live = (list: ChannelCard[]) => list.filter((c) => c.stream);
  const offline = (list: ChannelCard[]) => list.filter((c) => !c.stream);
</script>

{#snippet channels(list: ChannelCard[])}
  {#if grid}
    {#if live(list).length}
      <div class="cards">
        {#each live(list) as c (c.id)}
          <StreamCard card={c} {nav} oncategory={category} ontag={tag} />
        {/each}
      </div>
    {/if}
    {#if offline(list).length}
      <div class="tiles">
        {#each offline(list) as c (c.id)}
          <ChannelTile card={c} {nav} />
        {/each}
      </div>
    {/if}
  {:else}
    <div class="rows">
      {#each list as c (c.id)}
        <ChannelRow card={c} {nav} />
      {/each}
    </div>
  {/if}
{/snippet}

<div class="home">
  <header>
    <h1>Following</h1>
    {#if owner}
      <button class="account" disabled={home.able.length < 2} onclick={pickAccount}>
        <Avatar src={owner.avatar} name={owner.displayName} size={20} />
        <span>{owner.displayName}</span>
        {#if home.able.length > 1}<Icon name="chevronDown" size={14} />{/if}
      </button>
    {/if}
    <SearchField small width="min(260px, 40%)" placeholder="Search…" label="Search channels" bind:value={query} />
    <span class="grow"></span>
    <IconButton icon="sliders" label="Sort" hint="Order of live and offline channels." onclick={sortMenu} />
    <IconButton icon="grid" label="Grid" hint="Cards with preview pictures." active={grid} toggle onclick={() => setLayout('grid')} />
    <IconButton icon="list" label="List" hint="One row per channel." active={!grid} toggle onclick={() => setLayout('list')} />
    <IconButton icon="refresh" label="Refresh" hint="Load who you follow again." disabled={home.loading} onclick={() => void home.refresh(true)} />
  </header>

  <div class="scroll" use:keepScroll={'home'}>
    {#if !home.account}
      <div class="note">
        <span>Sign in to see the channels you follow.</span>
        <Button onclick={onsignin}>Sign in</Button>
      </div>
    {:else if home.error}
      <div class="note">
        <span>{home.error}</span>
        <Button onclick={() => void home.refresh(true)}>Try again</Button>
      </div>
    {/if}

    {#if lists.favorites.length}
      <section>
        <h2><Icon name="starFilled" size={13} />Favorites</h2>
        {@render channels(lists.favorites)}
      </section>
    {/if}

    {#if home.follows}
      {#if lists.live.length}
        <section>
          <h2><span class="dot"></span>Live <span class="count">{lists.live.length}</span></h2>
          {@render channels(lists.live)}
        </section>
      {/if}
      {#if !lists.live.length && lists.offline.length && !query.trim()}
        <div class="quiet">
          <span>No one you follow is live.</span>
          <Button small onclick={ondiscover}>Find streams</Button>
        </div>
      {/if}
      {#if lists.offline.length}
        <section>
          <button class="fold" aria-expanded={offlineOpen} onclick={() => (offlineOpen = !offlineOpen)}>
            <h2>
              <Icon name={offlineOpen ? 'chevronDown' : 'chevronRight'} size={14} />Offline
              <span class="count">{lists.offline.length}</span>
            </h2>
          </button>
          {#if offlineOpen}{@render channels(lists.offline)}{/if}
        </section>
      {/if}
      {#if !lists.live.length && !lists.offline.length && !lists.favorites.length}
        <div class="empty">
          {#if query.trim()}
            Nothing matches “{query.trim()}”.
          {:else}
            <span>You don’t follow anyone yet.</span>
            <Button onclick={ondiscover}>Find streams</Button>
          {/if}
        </div>
      {/if}
    {:else if home.account && home.loading}
      <Loading rows={8} />
    {:else if !home.account && !lists.favorites.length}
      <div class="empty">
        <Icon name="star" size={22} />
        <span>Favorite channels in Discover to keep them here.</span>
      </div>
    {/if}
  </div>
  <PickedBar {nav} />
  <Peek />
</div>

<style>
  .home {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 20px;
    border-bottom: 1px solid var(--border);
  }
  h1 {
    margin: 0 6px 0 0;
    font-size: 1.05rem;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px 0 4px;
    background: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--text);
    font-size: 0.8rem;
  }
  .account:disabled {
    opacity: 1;
    cursor: default;
  }
  .account:not(:disabled):hover {
    background: var(--surface-hover);
  }
  .grow {
    flex: 1;
  }
  .quiet {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 28px 0 8px;
    color: var(--dim);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 20px 72px;
  }
  section {
    margin-top: 14px;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 0 0 10px;
    font-size: 0.85rem;
    font-weight: 700;
    color: var(--muted);
  }
  .count {
    color: var(--dim);
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    background: var(--live);
    border-radius: 50%;
  }
  .fold {
    padding: 0;
    background: none;
    border: 0;
    color: inherit;
  }
  .fold:hover h2 {
    color: var(--text);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 20px 14px;
    margin-bottom: 14px;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    gap: 10px;
  }
  .tiles > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: auto 130px;
  }
  .rows {
    display: flex;
    flex-direction: column;
  }
  .rows > :global(*) {
    content-visibility: auto;
    contain-intrinsic-size: auto 36px;
  }
  .note {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 12px;
    padding: 10px 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.85rem;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 64px 0;
    color: var(--dim);
  }
</style>
