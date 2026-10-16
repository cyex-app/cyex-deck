<!-- Discover (1.7): live streams, categories and search. Tags, the remembered language
     filter and your followed categories are H4. -->
<script lang="ts">
  import { onDestroy } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import IconButton from '../../components/IconButton.svelte';
  import { openMenuAt } from '../../components/menu.svelte';
  import Segmented from '../../components/Segmented.svelte';
  import type { Category } from '../../ipc/bindings/Category';
  import type { GameRef } from '../../ipc/bindings/GameRef';
  import type { StreamFilter } from '../../ipc/bindings/StreamFilter';
  import type { StreamSort } from '../../ipc/bindings/StreamSort';
  import { discoverIpc } from '../../ipc/browse';
  import { accounts } from '../../lib/accounts.svelte';
  import { formatShort } from '../../lib/format';
  import { settings } from '../../lib/settings.svelte';
  import type { Nav } from '../browse/open.svelte';
  import Peek from '../browse/Peek.svelte';
  import PickedBar from '../browse/PickedBar.svelte';
  import CategoryList from './CategoryList.svelte';
  import { discover } from './discover.svelte';
  import LanguagePicker from './LanguagePicker.svelte';
  import SearchResults from './SearchResults.svelte';
  import StreamList from './StreamList.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { keepScroll } from '../../lib/keepScroll';

  let { onopen }: { onopen: (tabId: string) => void } = $props();

  const nav: Nav = { show: (id) => onopen(id), account: () => accounts.current?.userId ?? null };

  const SORTS: [StreamSort, string][] = [
    ['viewers', 'Most viewers'],
    ['viewersAsc', 'Fewest viewers'],
    ['recent', 'Started recently'],
  ];
  const TABS = [
    { value: 'streams', label: 'Streams' },
    { value: 'categories', label: 'Categories' },
  ] as const;

  const place = $derived(discover.place);
  let typed = $state(discover.place.kind === 'search' ? discover.place.text : '');
  let timer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(timer));
  /** The category on screen, once Twitch says what it is. */
  let info = $state<Category | null>(null);

  const filter = $derived<StreamFilter>({
    sort: discover.sort,
    languages: settings.v.browse.languages,
    tag: discover.tag,
  });
  const filterKey = $derived(JSON.stringify(filter));
  const lists = $derived(place.kind === 'streams' || place.kind === 'category');
  /** Neither is lit while searching. */
  const tabValue = $derived((place.kind === 'search' ? '' : place.kind === 'categories' ? 'categories' : 'streams') as
    | 'streams'
    | 'categories');

  // Leaving search from elsewhere (back, a category) empties the box.
  $effect(() => {
    if (place.kind !== 'search') typed = '';
  });

  function search(text: string, now = false) {
    clearTimeout(timer);
    const run = () => {
      const q = text.trim();
      if (q) discover.go({ kind: 'search', text: q });
      else if (place.kind === 'search') discover.back();
    };
    if (now) run();
    else timer = setTimeout(run, 350);
  }

  function sortMenu(e: MouseEvent) {
    openMenuAt(
      e,
      SORTS.map(([value, label]) => ({ label, checked: discover.sort === value, run: () => (discover.sort = value) })),
    );
  }

  const category = (game: GameRef) => discover.go({ kind: 'category', slug: game.slug, name: game.name });
  const tag = (t: string) => discover.showTag(t);

  async function loadCategory(slug: string, after: string | null) {
    const found = await discoverIpc.category(slug, filter, after);
    if (!found) throw new Error('That category is gone.');
    if (!after) info = found.category;
    return found.streams;
  }
</script>

<div class="discover">
  <header>
    {#if discover.canGoBack}
      <IconButton icon="chevronLeft" label="Back" onclick={() => discover.back()} />
    {/if}
    <SearchField
      small
      width="min(340px, 45%)"
      placeholder="Search channels and categories"
      bind:value={typed}
      oninput={() => search(typed)}
      onkeydown={(e) => e.key === 'Enter' && search(typed, true)}
      onclear={() => search('', true)}
    />
    <Segmented
      label="Show"
      options={TABS}
      value={tabValue}
      onchange={(v) => place.kind !== v && discover.go({ kind: v })}
    />
    <span class="grow"></span>
    {#if lists}
      <button class="menu" onclick={sortMenu}>
        {SORTS.find(([v]) => v === discover.sort)?.[1]}<Icon name="chevronDown" size={13} />
      </button>
      <LanguagePicker />
    {/if}
  </header>

  <div class="scroll" use:keepScroll={'discover'}>
    {#if lists && discover.tag}
      <div class="chips">
        <button class="chip" aria-label="Show all tags" onclick={() => (discover.tag = null)}>
          {discover.tag}<Icon name="x" size={12} />
        </button>
      </div>
    {/if}

    {#if place.kind === 'streams'}
      <StreamList
        key="streams {filterKey}"
        load={(after) => discoverIpc.streams(filter, after)}
        {nav}
        oncategory={category}
        ontag={tag}
      />
    {:else if place.kind === 'categories'}
      <CategoryList />
    {:else if place.kind === 'category'}
      {@const slug = place.slug}
      <div class="cat-head">
        {#if info?.slug === slug && info.boxArt}<img src={info.boxArt} alt="" />{/if}
        <div>
          <h1>{info?.slug === slug ? info.name : place.name}</h1>
          {#if info?.slug === slug}
            <span class="sub">
              {formatShort(info.viewers)} viewers{info.followers ? ` · ${formatShort(info.followers)} followers` : ''}
            </span>
          {/if}
        </div>
      </div>
      <StreamList
        key="category {slug} {filterKey}"
        load={(after) => loadCategory(slug, after)}
        {nav}
        ontag={tag}
        showGame={false}
      />
    {:else}
      <SearchResults text={place.text} {nav} />
    {/if}
  </div>
  <PickedBar {nav} />
  <Peek />
</div>

<style>
  .discover {
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
  .grow {
    flex: 1;
  }
  .menu {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 28px;
    padding: 0 8px;
    background: var(--surface);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.8rem;
    white-space: nowrap;
  }
  .menu:hover {
    color: var(--text);
  }
  .scroll {
    flex: 1;
    overflow-y: auto;
    padding: 16px 20px 72px;
  }
  .chips {
    display: flex;
    gap: 6px;
    margin-bottom: 14px;
  }
  .chip {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 24px;
    padding: 0 8px 0 10px;
    background: var(--accent-dim);
    border: 1px solid var(--accent-border);
    border-radius: var(--radius-pill);
    color: var(--text);
    font-size: 0.8rem;
  }
  .cat-head {
    display: flex;
    align-items: center;
    gap: 14px;
    margin-bottom: 18px;
  }
  .cat-head img {
    width: 72px;
    height: 96px;
    border-radius: var(--radius);
    object-fit: cover;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 1.2rem;
  }
  .sub {
    color: var(--dim);
    font-size: 0.85rem;
  }
</style>
