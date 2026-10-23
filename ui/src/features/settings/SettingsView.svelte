<!-- Settings (spec 12), in the main window only: they don't pop out (V103-08). The list is
     in groups (V101-07); narrow windows show icons only, with lines between the groups.
     Search finds any setting in the registry and jumps to it (V103-13). -->
<script lang="ts">
  import { onDestroy, tick, untrack, type Component } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { ACTIONS } from '../../lib/hotkeys.svelte';
  import { quietScroll } from '../../lib/quietScroll';
  import { settings } from '../../lib/settings.svelte';
  import { ENTRIES, GROUPS, hotkeyEntries, PAGES, type PageId } from './registry/index';
  import { reveal } from './reveal';
  import { buildIndex, search, type Hit } from './search';
  import { settingsPlace } from './place.svelte';
  import { countSearch, PAUSE_MS, removeRecent } from './recent';
  import RecentSearches from './RecentSearches.svelte';
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
  import MinerSection from './sections/MinerSection.svelte';
  import NotificationsSection from './sections/NotificationsSection.svelte';
  import PlayerSection from './sections/PlayerSection.svelte';
  import PluginsSection from './sections/PluginsSection.svelte';
  import RecordingsSection from './sections/RecordingsSection.svelte';
  import SoundSection from './sections/SoundSection.svelte';
  import StreamManagerSection from './sections/StreamManagerSection.svelte';
  import StreamerSection from './sections/StreamerSection.svelte';
  import WindowsSection from './sections/WindowsSection.svelte';
  import UploaderSection from './sections/UploaderSection.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { forgetScroll, keepScroll } from '../../lib/keepScroll';
  import { keyTaken } from '../../lib/escape';
  import { isFindKey, visibleSearchbox } from '../../lib/findKey';
  import { recording } from '../../lib/hotkeys.svelte';

  interface Props {
    /** Open page id. Bindable so other views can jump to a page. */
    section?: string;
    /** S9 Leaves Settings for the view before it. Escape calls it once the search is clear. */
    onclose?: () => void;
  }

  let { section = $bindable('general'), onclose }: Props = $props();

  const VIEWS: Record<PageId, Component> = {
    general: GeneralSection,
    appearance: AppearanceSection,
    windows: WindowsSection,
    sound: SoundSection,
    hotkeys: HotkeysSection,
    plugins: PluginsSection,
    data: DataSection,
    accounts: AccountsSection,
    notifications: NotificationsSection,
    chat: ChatSection,
    emotes: EmotesSection,
    highlights: HighlightsSection,
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
  // V107-Mo Back on the page you left: search and scroll come back. Sent to another page: its top.
  if (!settingsPlace.arrive(shown)) forgetScroll('settings');
  $effect(() => {
    if (section !== shown) {
      shown = section;
      settingsPlace.page = section;
      // A jump from outside (Ctrl+K, a menu) shows the page, not an old search.
      query = '';
      // Another page opens at the top.
      if (content) content.scrollTop = 0;
      void settings.pruneBlank();
    }
  });
  onDestroy(() => void settings.pruneBlank());

  const docs = buildIndex([...ENTRIES, ...hotkeyEntries(ACTIONS)], PAGES);
  // V107-Mo Coming back to Settings brings the search back (place.svelte.ts).
  let query = $state(settingsPlace.query);
  $effect(() => {
    settingsPlace.query = query;
  });
  /** Which box has focus (side or top), for the recent list. */
  let focused = $state('');
  const recent = $derived(settings.v.general.recentSearches ?? []);
  /** The recent search the arrow keys are on; -1 is the box. */
  let pickRecent = $state(-1);
  /** Escape put the list away until you type or come back to the box. */
  let recentAway = $state(false);
  const recentShown = (where: string) => focused === where && !query && !recentAway && recent.length > 0;
  /** What this search already added after a pause; typing on replaces it. */
  let counted = untrack(() => query);
  let active = $state(0);
  let content = $state<HTMLElement>();
  let nav = $state<HTMLElement>();
  let root = $state<HTMLElement>();
  const searching = $derived(query.trim() !== '');
  const hits = $derived(search(docs, query));

  function remember(list: string[] | null) {
    if (list) void settings.patch('general', { recentSearches: list });
  }

  /** V107-Mo A search counts once you paused on it with results, or picked one; once each. */
  function count(q: string) {
    remember(countSearch(untrack(() => recent), q, counted));
    counted = q;
  }

  $effect(() => {
    const q = query;
    if (!q.trim()) counted = '';
    if (!q.trim() || !hits.length) return;
    const t = setTimeout(() => count(q), PAUSE_MS);
    return () => clearTimeout(t);
  });

  function pickFromRecent(q: string) {
    query = q;
    active = 0;
    pickRecent = -1;
  }

  /** The box in a `.box`, back in focus (Clear or Escape from the list's own button). */
  const boxInput = (el: Element | null) => el?.closest('.box')?.querySelector<HTMLInputElement>('input');

  function removeFromRecent(q: string) {
    const left = removeRecent(recent, q);
    pickRecent = Math.min(pickRecent, left.length - 1);
    remember(left);
  }

  /** The box as a combobox over the recent list: screen readers hear the arrow pick. */
  function combo(box: HTMLElement, p: { id: string; open: boolean; active: number }) {
    const input = box.querySelector('input');
    const apply = (c: typeof p) => {
      if (!input) return;
      input.setAttribute('role', 'combobox');
      input.setAttribute('aria-autocomplete', 'list');
      input.setAttribute('aria-expanded', String(c.open));
      if (c.open) {
        input.setAttribute('aria-controls', c.id);
        if (c.active >= 0) input.setAttribute('aria-activedescendant', `${c.id}-${c.active}`);
        else input.removeAttribute('aria-activedescendant');
      } else {
        input.removeAttribute('aria-controls');
        input.removeAttribute('aria-activedescendant');
      }
    };
    apply(p);
    return { update: apply };
  }

  /** Arrows, Enter, Delete and Escape for the recent list, before the box sees them. */
  function onrecentkey(e: KeyboardEvent, where: string) {
    if (!recentShown(where) || e.isComposing) return;
    const n = recent.length;
    const inBox = e.target instanceof HTMLInputElement;
    if (!inBox && e.key !== 'Escape') return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      pickRecent = e.key === 'ArrowDown' ? Math.min(n - 1, pickRecent + 1) : Math.max(-1, pickRecent - 1);
    } else if (e.key === 'Enter' && pickRecent >= 0) {
      pickFromRecent(recent[pickRecent]);
    } else if (e.key === 'Delete' && pickRecent >= 0) {
      removeFromRecent(recent[pickRecent]);
    } else if (e.key === 'Escape') {
      recentAway = true;
      pickRecent = -1;
      if (!inBox) boxInput(e.target as Element)?.focus();
    } else return;
    e.preventDefault();
    e.stopPropagation();
  }

  function open(id: string) {
    query = '';
    section = id;
  }

  async function jump(hit: Hit) {
    count(query);
    open(hit.entry.page);
    await tick();
    if (content) reveal(content, hit.entry.anchor ? [hit.entry.id, hit.entry.anchor] : [hit.entry.id]);
  }

  const FIELD = 'input, textarea, select, [contenteditable="true"]';

  /** S9 Escape: the search first, then Settings. A field you're typing in just lets go. */
  function onescape(e: KeyboardEvent) {
    if (e.key !== 'Escape' || e.isComposing || keyTaken(e) || recording.active) return;
    const el = e.target instanceof HTMLElement ? e.target : null;
    if (query) {
      e.preventDefault();
      query = '';
      return;
    }
    if (el?.matches(FIELD) && !el.closest('.box')) {
      el.blur();
      return;
    }
    e.preventDefault();
    onclose?.();
  }

  /** Ctrl+F: the settings search takes focus, text selected. */
  function onfind(e: KeyboardEvent) {
    if (!isFindKey(e) || e.isComposing || keyTaken(e) || recording.active) return;
    const box = visibleSearchbox(root ?? document);
    if (!box) return;
    e.preventDefault();
    box.focus();
    box.select();
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      active = Math.max(0, Math.min(hits.length - 1, active + (e.key === 'ArrowDown' ? 1 : -1)));
    } else if (e.key === 'Enter' && hits[active]) {
      e.preventDefault();
      void jump(hits[active]);
    }
  }
</script>

<svelte:window onkeydown={(e) => (onescape(e), onfind(e))} />

<!-- One box, drawn in the list or, in narrow windows, above the page. -->
{#snippet searchBox(where: string)}
  <div
    class="box {where}"
    role="presentation"
    onfocusin={() => ((focused = where), (recentAway = false))}
    onfocusout={(e) => {
      // Tab from the box into the list (Clear) stays inside.
      if (e.relatedTarget instanceof Node && e.currentTarget.contains(e.relatedTarget)) return;
      if (focused === where) (focused = ''), (pickRecent = -1);
    }}
    use:combo={{ id: `recent-${where}`, open: recentShown(where), active: pickRecent }}
    onkeydowncapture={(e) => onrecentkey(e, where)}
  >
    <SearchField
      placeholder="Search settings"
      bind:value={query}
      oninput={() => ((active = 0), (recentAway = false), (pickRecent = -1))}
      onkeydown={onkey}
    />
    {#if recentShown(where)}
      <RecentSearches
        id="recent-{where}"
        items={recent}
        active={pickRecent}
        onpick={pickFromRecent}
        onremove={(q) => ((pickRecent = -1), remember(removeRecent(recent, q)))}
        onclear={() => (remember([]), boxInput(document.activeElement)?.focus())}
      />
    {/if}
  </div>
{/snippet}

<div class="settings" bind:this={root}>
  <nav bind:this={nav} use:quietScroll>
    {@render searchBox('side')}
    {#each GROUPS as g (g.label)}
      <div class="group">{g.label}</div>
      {#each g.pages as p (p.id)}
        <button class:on={!searching && section === p.id} aria-current={!searching && section === p.id ? 'page' : undefined} onclick={() => open(p.id)} use:tooltip={{ label: p.label, when: () => (nav?.offsetWidth ?? 999) < 100 }}>
          <Icon name={p.icon} size={15} />
          <span>{p.label}</span>
        </button>
      {/each}
    {/each}
  </nav>
  <div class="content" bind:this={content} use:keepScroll={'settings'}>
    <div class="inner">
      {@render searchBox('top')}
      {#if searching}
        <SearchResults {hits} {query} {active} onpick={(hit) => void jump(hit)} onhover={(i) => (active = i)} />
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
  /* S7 The list's scrollbar shows on hover or while scrolling (quietScroll). */
  nav::-webkit-scrollbar {
    width: 6px;
  }
  .box {
    position: relative;
    flex: none;
  }
  .box.side {
    margin: 8px 0 2px;
  }
  .box.top {
    display: none;
    margin-bottom: 16px;
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
    /* The column stays put when a page is short enough to need no scrollbar. */
    scrollbar-gutter: stable;
  }
  /* S5, S6 One column width on every page, centred in the room beside the list. */
  .inner {
    box-sizing: border-box;
    max-width: 760px;
    margin-inline: auto;
    padding: 22px 28px 32px;
  }
  /* C5 An example in an empty field never reads like a saved value. */
  .content :global(input::placeholder),
  .content :global(textarea::placeholder) {
    color: var(--dim);
    opacity: 0.6;
  }
  /* rb-3 The style only: the `text-wrap` shorthand also set the wrap mode, which beat a
     line's own `white-space: nowrap` (the OBS folder path). */
  .inner :global(p) {
    text-wrap-style: pretty;
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
    transition: box-shadow calc(var(--slow) * 2) var(--ease);
  }
  /* Narrow windows: icons only (3.10), search above the page. */
  @container (max-width: 600px) {
    nav {
      width: 52px;
      padding-inline: 8px;
    }
    nav button span,
    .box.side {
      display: none;
    }
    .box.top {
      display: block;
    }
    .group {
      height: 1px;
      margin: 8px 4px;
      padding: 0;
      overflow: hidden;
      background: var(--border);
      color: transparent;
    }
    .box.side + .group {
      display: none;
    }
    .inner {
      padding: 18px 16px 28px;
    }
  }
</style>
