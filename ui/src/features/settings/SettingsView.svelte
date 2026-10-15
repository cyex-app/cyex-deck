<!-- Settings (spec 12), in the main window only: they don't pop out (V103-08). The list is
     in groups (V101-07); narrow windows show icons only, with lines between the groups.
     Search finds any setting in the registry and jumps to it (V103-13). -->
<script lang="ts">
  import { onDestroy, tick, untrack, type Component } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { ACTIONS } from '../../lib/hotkeys.svelte';
  import { settings } from '../../lib/settings.svelte';
  import { ENTRIES, GROUPS, hotkeyEntries, PAGES, type PageId } from './registry/index';
  import { reveal } from './reveal';
  import { buildIndex, search, type Hit } from './search';
  import SearchResults from './SearchResults.svelte';
  import AccountsSection from './sections/AccountsSection.svelte';
  import AppearanceSection from './sections/AppearanceSection.svelte';
  import AutomationsSection from './sections/AutomationsSection.svelte';
  import ChatSection from './sections/ChatSection.svelte';
  import CommandsSection from './sections/CommandsSection.svelte';
  import DataSection from './sections/DataSection.svelte';
  import EmotesSection from './sections/EmotesSection.svelte';
  import FiltersSection from './sections/FiltersSection.svelte';
  import GeneralSection from './sections/GeneralSection.svelte';
  import HighlightsSection from './sections/HighlightsSection.svelte';
  import HotkeysSection from './sections/HotkeysSection.svelte';
  import IgnoresSection from './sections/IgnoresSection.svelte';
  import MentionsSection from './sections/MentionsSection.svelte';
  import MinerSection from './sections/MinerSection.svelte';
  import NotificationsSection from './sections/NotificationsSection.svelte';
  import PlayerSection from './sections/PlayerSection.svelte';
  import PluginsSection from './sections/PluginsSection.svelte';
  import RecordingsSection from './sections/RecordingsSection.svelte';
  import StreamManagerSection from './sections/StreamManagerSection.svelte';
  import StreamerSection from './sections/StreamerSection.svelte';
  import WindowsSection from './sections/WindowsSection.svelte';
  import UploaderSection from './sections/UploaderSection.svelte';

  interface Props {
    /** Open page id. Bindable so other views can jump to a page. */
    section?: string;
  }

  let { section = $bindable('general') }: Props = $props();

  const VIEWS: Record<PageId, Component> = {
    general: GeneralSection,
    appearance: AppearanceSection,
    windows: WindowsSection,
    hotkeys: HotkeysSection,
    plugins: PluginsSection,
    data: DataSection,
    accounts: AccountsSection,
    notifications: NotificationsSection,
    chat: ChatSection,
    emotes: EmotesSection,
    highlights: HighlightsSection,
    mentions: MentionsSection,
    ignores: IgnoresSection,
    filters: FiltersSection,
    commands: CommandsSection,
    uploads: UploaderSection,
    automations: AutomationsSection,
    recordings: RecordingsSection,
    player: PlayerSection,
    miner: MinerSection,
    streammanager: StreamManagerSection,
    streamer: StreamerSection,
  };

  const Section = $derived(VIEWS[section as PageId] ?? GeneralSection);

  // V105-12 Leaving a page (or Settings) clears the rules left empty; never while you can still type in them.
  let shown = untrack(() => section);
  $effect(() => {
    if (section !== shown) {
      shown = section;
      void settings.pruneBlank();
    }
  });
  onDestroy(() => void settings.pruneBlank());

  const docs = buildIndex([...ENTRIES, ...hotkeyEntries(ACTIONS)], PAGES);
  let query = $state('');
  let active = $state(0);
  let content = $state<HTMLElement>();
  const searching = $derived(query.trim() !== '');
  const hits = $derived(search(docs, query));

  function open(id: string) {
    query = '';
    section = id;
  }

  async function jump(hit: Hit) {
    open(hit.entry.page);
    await tick();
    if (content) reveal(content, hit.entry.anchor ? [hit.entry.id, hit.entry.anchor] : [hit.entry.id]);
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(0, Math.min(hits.length - 1, active + (e.key === 'ArrowDown' ? 1 : -1)));
    } else if (e.key === 'Enter' && hits[active]) {
      e.preventDefault();
      void jump(hits[active]);
    } else if (e.key === 'Escape' && query) {
      e.stopPropagation();
      query = '';
    }
  }
</script>

<!-- One box, drawn in the list or, in narrow windows, above the page. -->
{#snippet searchBox(where: string)}
  <div class="search {where}">
    <Icon name="search" size={14} />
    <input
      class="field"
      type="search"
      placeholder="Search settings"
      aria-label="Search settings"
      spellcheck="false"
      bind:value={query}
      oninput={() => (active = 0)}
      onkeydown={onkey}
    />
    {#if query}
      <button class="clear" aria-label="Clear search" onclick={() => (query = '')}><Icon name="x" size={12} /></button>
    {/if}
  </div>
{/snippet}

<div class="settings">
  <nav>
    {@render searchBox('side')}
    {#each GROUPS as g (g.label)}
      <div class="group">{g.label}</div>
      {#each g.pages as p (p.id)}
        <button class:on={!searching && section === p.id} onclick={() => open(p.id)} title={p.label}>
          <Icon name={p.icon} size={15} />
          <span>{p.label}</span>
        </button>
      {/each}
    {/each}
  </nav>
  <div class="content" bind:this={content}>
    <div class="inner">
      {@render searchBox('top')}
      {#if searching}
        <SearchResults {hits} {query} {active} onpick={(hit) => void jump(hit)} />
      {:else}
        <Section />
      {/if}
    </div>
  </div>
</div>

<style>
  .settings {
    flex: 1;
    display: flex;
    min-height: 0;
    container-type: inline-size;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 2px;
    width: 210px;
    flex: none;
    padding: 4px 10px 14px;
    overflow-y: auto;
    border-right: 1px solid var(--border);
  }
  .search {
    position: relative;
    display: flex;
    align-items: center;
    flex: none;
  }
  .search.side {
    margin: 8px 0 2px;
  }
  .search.top {
    display: none;
    margin-bottom: 16px;
  }
  .search > :global(svg) {
    position: absolute;
    left: 9px;
    color: var(--dim);
    pointer-events: none;
  }
  .search input {
    width: 100%;
    padding: 0 28px 0 29px;
  }
  .search input::-webkit-search-cancel-button {
    display: none;
  }
  .clear {
    position: absolute;
    right: 4px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
  }
  .clear::before {
    content: "";
    position: absolute;
    inset: -1px;
  }
  .clear:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .group {
    padding: 12px 10px 4px;
    color: var(--dim);
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  nav button {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
    height: 30px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-weight: 600;
    text-align: left;
    white-space: nowrap;
  }
  nav button:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  nav button.on {
    background: var(--accent-dim);
    color: var(--accent-text);
  }
  .content {
    flex: 1;
    overflow-y: auto;
  }
  .inner {
    max-width: 680px;
    padding: 22px 28px 32px;
  }
  /* V103-13 A setting search jumped to: tinted over its own background, then fades. */
  .content :global(.found),
  .content :global(.fading) {
    border-radius: var(--radius);
  }
  .content :global(.found) {
    box-shadow:
      inset 0 0 0 100vmax var(--accent-dim),
      0 0 0 4px var(--accent-dim);
  }
  .content :global(.fading) {
    box-shadow:
      inset 0 0 0 100vmax transparent,
      0 0 0 4px transparent;
    transition: box-shadow 0.7s var(--ease);
  }
  /* Narrow windows: icons only (3.10), search above the page. */
  @container (max-width: 600px) {
    nav {
      width: 52px;
      padding-inline: 8px;
    }
    nav button span,
    .search.side {
      display: none;
    }
    .search.top {
      display: flex;
    }
    .group {
      height: 1px;
      margin: 8px 4px;
      padding: 0;
      overflow: hidden;
      background: var(--border);
      color: transparent;
    }
    .search.side + .group {
      display: none;
    }
    .inner {
      padding: 18px 16px 28px;
    }
  }
</style>
