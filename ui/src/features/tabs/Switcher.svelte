<!-- U1 Quick switcher (Ctrl+K): jump to a tab or view, or open any channel. Followed
     channels (1.6) show once something is typed. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import { closeMenu } from '../../components/menu.svelte';
  import SearchField from '../../components/SearchField.svelte';
  import type { IconName } from '../../components/icons';
  import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { discoverIpc } from '../../ipc/browse';
  import { tabsIpc } from '../../ipc/tabs';
  import { accounts } from '../../lib/accounts.svelte';
  import { layer } from '../../lib/escape';
  import { firstField, focusScope } from '../../lib/focusScope';
  import { scrollBehavior } from '../../lib/motion';
  import { FLAGS, lockFor } from '../../lib/gate';
  import { gate } from '../../lib/gate.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { openChannel } from '../browse/open.svelte';
  import { rank } from '../chat/completion';
  import { home } from '../home/home.svelte';
  import { mods } from '../modview/moderation.svelte';
  import { modviewPlace } from '../modview/where.svelte';
  import { PAGES } from '../settings/registry/pages';
  import { liveOrExact } from './channelMatch';
  import { openAdd } from './dialog.svelte';
  import { label, tabs, tiles } from './tabs.svelte';
  import { errorText } from '../../lib/errors';
  import { scrollWithin } from '../../lib/scrollWithin';

  interface Props {
    onopen: (id: string) => void;
    onview: (view: View) => void;
    /** Opens Settings on a page. */
    onsettings: (page: string) => void;
    onclose: () => void;
  }

  let { onopen, onview, onsettings, onclose }: Props = $props();

  interface Choice {
    key: string;
    label: string;
    hint?: string;
    icon?: IconName;
    live?: 'live' | 'rerun' | null;
    /** How well it matches what's typed (`rank`). */
    score?: number;
    run: () => void;
  }

  type View = (typeof VIEWS)[number]['view'];

  /** The fixed views, each only while the title bar shows it (U11). */
  const VIEWS = [
    { view: 'home', label: 'Home', icon: 'house', shown: () => true },
    { view: 'discover', label: 'Discover', icon: 'compass', shown: () => true },
    // V102-20 Greyed out in the title bar without a GQL sign-in; here it says so.
    { view: 'miner', label: 'Miner', icon: 'pickaxe', shown: () => true },
    { view: 'streamManager', label: 'Stream manager', icon: 'radio', shown: () => accounts.any('streamManager') },
    { view: 'whispers', label: 'Whispers', icon: 'whisper', shown: () => accounts.can('whispers') },
    { view: 'cosmetics', label: '7TV paint and badge', icon: 'seventv', shown: () => accounts.list.length > 0 },
    { view: 'settings', label: 'Settings', icon: 'settings', shown: () => true },
  ] as const;

  let query = $state('');
  let index = $state(0);
  let box = $state<HTMLElement>();
  let list = $state<HTMLElement>();

  /** Followed channels shown at most. */
  const FOLLOWED = 8;
  /** Channels from search shown at most. */
  const FOUND = 5;

  // Live channels from Twitch's search (and the exact name, offline too), after a short pause.
  let found = $state<ChannelCard[]>([]);
  $effect(() => {
    const q = query.trim();
    if (q.length < 2) {
      found = [];
      return;
    }
    const timer = setTimeout(() => {
      discoverIpc
        .search(q)
        .then((r) => {
          if (q === query.trim()) found = liveOrExact(r.channels.items, q);
        })
        .catch(() => {});
    }, 250);
    return () => clearTimeout(timer);
  });

  // The shared dialog layer: one Escape (after clearing the text), Tab stays inside, focus goes back.
  $effect(() => {
    if (!box) return;
    // Only one thing floats at a time: a menu that was open goes.
    closeMenu();
    const release = focusScope(box, { trap: true, initial: firstField });
    const off = layer('dialog', onclose);
    return () => {
      off();
      release();
    };
  });

  // Arrowing past the visible rows keeps the active one in view.
  $effect(() => {
    void index;
    const row = list?.querySelector<HTMLElement>('[aria-selected="true"]');
    if (row) scrollWithin(row, 'nearest', scrollBehavior());
  });
  onMount(() => home.ensure());

  const choices = $derived.by((): Choice[] => {
    const q = query.trim().toLowerCase();
    const typed = query.trim();
    const score = (...names: string[]) => Math.max(...names.map((n) => rank(n, typed).rank));
    const tabChoices = tabs.list
      .map((t): Choice => ({
        key: t.id,
        label: label(t),
        hint: hint(t),
        live: tabs.liveState(t),
        score: q ? score(label(t), ...tiles(t).map((x) => x.channel.login)) : 0,
        run: () => onopen(t.id),
      }))
      .filter((c) => !q || c.score);
    // Live tabs first, then the bar's order.
    tabChoices.sort((a, b) => Number(!!b.live) - Number(!!a.live));
    const views = VIEWS.filter((v) => q && score(v.label) && v.shown()).map((v): Choice => {
      const lock = v.view === 'miner' ? lockFor(accounts.any('miner'), 'gql') : null;
      const base = { key: v.view, label: v.label, icon: v.icon, score: score(v.label) };
      return lock
        ? { ...base, hint: FLAGS[lock].label, run: () => gate.ask(lock) }
        : { ...base, run: () => onview(v.view) };
    });
    // Mod view needs a channel: one entry per channel in the tabs that an account moderates.
    const modviews = q
      ? [...new Set(tabs.list.flatMap((t) => tiles(t).map((x) => x.channel.login)))]
          .filter((login) => mods.moderating(login) && score('Mod view', login))
          // A step under a tab or channel that matches as well: "mod" finds the Mod view of the
          // channels you moderate, but a channel called "mod…" still comes first.
          .map((login): Choice => ({
            key: `m${login}`,
            label: `Mod view · ${login}`,
            icon: 'shield',
            score: score('Mod view', login) - 0.5,
            run: () => modviewPlace.open(login),
          }))
      : [];
    const pages = q
      ? PAGES.filter((p) => score(p.label)).map(
          (p): Choice => ({
            key: `s${p.id}`,
            label: p.label,
            hint: 'Settings',
            icon: p.icon,
            score: score(p.label),
            run: () => onsettings(p.id),
          }),
        )
      : [];
    const inTabs = new Set(tabs.list.flatMap((t) => tiles(t).map((x) => x.channel.id)));
    const followed = q
      ? (home.follows ?? [])
          .filter((c) => !inTabs.has(c.id) && score(c.login, c.name) > 0)
          .sort((a, b) => (b.stream?.viewers ?? -1) - (a.stream?.viewers ?? -1))
          .slice(0, FOLLOWED)
          .map(
            (c): Choice => ({
              key: `f${c.id}`,
              label: c.name,
              score: score(c.login, c.name),
              hint: c.stream ? (c.stream.game?.name ?? 'Live') : 'Followed',
              live: c.stream ? (c.stream.rerun ? 'rerun' : 'live') : null,
              icon: c.stream ? undefined : 'heart',
              run: () => void openChannel({ show: onopen, account: () => home.account }, c),
            }),
          )
      : [];
    const known = new Set([...inTabs, ...(home.follows ?? []).map((c) => c.id)]);
    const live = q
      ? found
          .filter((c) => !known.has(c.id) && score(c.login, c.name))
          .slice(0, FOUND)
          .map(
            (c): Choice => ({
              key: `l${c.id}`,
              label: c.name,
              score: score(c.login, c.name),
              hint: c.stream ? (c.stream.game?.name ?? 'Live') : 'Offline',
              live: c.stream ? (c.stream.rerun ? 'rerun' : 'live') : null,
              run: () => void openChannel({ show: onopen, account: () => home.account }, c),
            }),
          )
      : [];
    const exact = [...followed, ...live].some((c) => c.label.toLowerCase() === q);
    const open: Choice[] =
      q && !exact
        ? [{ key: '+', label: `Open ${query.trim()}`, hint: 'New tab', icon: 'plus', run: () => void openTyped() }]
        : [];
    // Best match first; ties keep tabs, then followed, views, settings pages, search.
    const rest = [...tabChoices, ...followed, ...views, ...modviews, ...pages, ...live];
    if (q) rest.sort((a, b) => (b.score ?? 0) - (a.score ?? 0));
    return [...rest, ...open];
  });

  $effect(() => {
    void choices;
    index = 0;
  });

  function hint(t: Tab): string | undefined {
    const stream = tiles(t)
      .map((x) => tabs.live[x.channel.id]?.stream)
      .find(Boolean);
    return stream ? (stream.game ?? 'Live') : undefined;
  }

  async function openTyped() {
    const input = query.trim();
    onclose();
    try {
      const found = await tabsIpc.lookup(input);
      if (found.length !== 1) {
        if (!found.length) toast.error('No channel by that name or id.');
        else openAdd(undefined, input);
        return;
      }
      const c = found[0];
      const id = await tabs.apply({
        op: 'add',
        channel: { id: c.id, login: c.login, name: c.name },
        show: tabs.lastShow,
        account: accounts.current?.userId ?? null,
        alias: null,
        into: null,
      });
      if (id) onopen(id);
    } catch (e) {
      toast.error(errorText(e));
    }
  }

  function run(c: Choice | undefined) {
    if (!c) return;
    onclose();
    c.run();
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const step = e.key === 'ArrowDown' ? 1 : -1;
      index = (index + step + choices.length) % Math.max(choices.length, 1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      run(choices[index]);
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="switcher" bind:this={box} role="dialog" aria-modal="true" aria-label="Quick switcher" tabindex="-1">
    <div class="search">
      <SearchField bind:value={query} placeholder="Go to a tab, view or channel" label="Go to" autofocus {onkeydown} />
    </div>
    {#if choices.length}
      <div class="list" bind:this={list} role="listbox" aria-label="Results">
        {#each choices as c, i (c.key)}
          <button
            class="choice"
            class:on={i === index}
            role="option"
            aria-selected={i === index}
            onmouseenter={() => (index = i)}
            onclick={() => run(c)}
          >
            {#if c.icon}<Icon name={c.icon} size={15} />
            {:else}<span class="dot {c.live ?? ''}"></span>{/if}
            <span class="text">{c.label}</span>
            {#if c.hint}<span class="hint">{c.hint}</span>{/if}
          </button>
        {/each}
      </div>
    {:else}
      <p class="empty">No tabs yet. Type a channel name to open one.</p>
    {/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: var(--z-dialog);
    display: grid;
    place-items: start center;
    padding-top: 10vh;
    background: var(--overlay);
  }
  /* In a short window the list gets shorter, so the box ends inside the window. */
  .switcher {
    display: flex;
    flex-direction: column;
    width: 460px;
    max-width: calc(100vw - 24px);
    max-height: calc(90vh - 12px);
    background: var(--elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow-dialog);
    overflow: hidden;
  }
  .search {
    flex: none;
    padding: 8px;
    border-bottom: 1px solid var(--border);
  }
  .list {
    flex: 0 1 auto;
    min-height: 0;
    max-height: 360px;
    overflow-y: auto;
    padding: 4px;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    height: 34px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    text-align: left;
    color: var(--text);
  }
  .choice.on {
    background: var(--surface-hover);
  }
  .text {
    flex: 1;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .hint {
    color: var(--dim);
    font-size: 0.78rem;
  }
  .dot {
    width: 7px;
    height: 7px;
    margin: 0 4px;
    border-radius: 50%;
    background: var(--border-strong);
  }
  .dot.live {
    background: var(--live);
  }
  .dot.rerun {
    background: var(--yellow);
  }
  .empty {
    margin: 0;
    padding: 14px;
    color: var(--muted);
  }
</style>
