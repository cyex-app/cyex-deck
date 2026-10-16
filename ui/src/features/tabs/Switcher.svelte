<!-- U1 Quick switcher (Ctrl+K): jump to a tab or view, or open any channel. Followed
     channels (1.6) show once something is typed. -->
<script lang="ts">
  import { onMount } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import type { IconName } from '../../components/icons';
  import type { Tab } from '../../ipc/bindings/Tab';
  import { tabsIpc } from '../../ipc/tabs';
  import { accounts } from '../../lib/accounts.svelte';
  import { FLAGS, lockFor } from '../../lib/gate';
  import { gate } from '../../lib/gate.svelte';
  import { toast } from '../../lib/toast.svelte';
  import { openChannel } from '../browse/open.svelte';
  import { home } from '../home/home.svelte';
  import { openAdd } from './dialog.svelte';
  import { label, tabs, tiles } from './tabs.svelte';
  import { errorText } from '../../lib/errors';

  interface Props {
    onopen: (id: string) => void;
    onview: (view: View) => void;
    onclose: () => void;
  }

  let { onopen, onview, onclose }: Props = $props();

  interface Choice {
    key: string;
    label: string;
    hint?: string;
    icon?: IconName;
    live?: 'live' | 'rerun' | null;
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
  let field = $state<HTMLInputElement>();

  /** Followed channels shown at most. */
  const FOLLOWED = 8;

  $effect(() => field?.focus());
  onMount(() => home.ensure());

  const choices = $derived.by((): Choice[] => {
    const q = query.trim().toLowerCase();
    const matches = (s: string) => s.toLowerCase().includes(q);
    const tabChoices = tabs.list
      .filter((t) => !q || matches(label(t)) || tiles(t).some((x) => matches(x.channel.login)))
      .map((t): Choice => ({
        key: t.id,
        label: label(t),
        hint: hint(t),
        live: tabs.liveState(t),
        run: () => onopen(t.id),
      }));
    // Live tabs first, then the bar's order.
    tabChoices.sort((a, b) => Number(!!b.live) - Number(!!a.live));
    const views = VIEWS.filter((v) => q && matches(v.label) && v.shown()).map((v): Choice => {
      const lock = v.view === 'miner' ? lockFor(accounts.any('miner'), 'gql') : null;
      return lock
        ? { key: v.view, label: v.label, icon: v.icon, hint: FLAGS[lock].label, run: () => gate.ask(lock) }
        : { key: v.view, label: v.label, icon: v.icon, run: () => onview(v.view) };
    });
    const inTabs = new Set(tabs.list.flatMap((t) => tiles(t).map((x) => x.channel.id)));
    const followed = q
      ? (home.follows ?? [])
          .filter((c) => !inTabs.has(c.id) && (matches(c.login) || matches(c.name)))
          .sort((a, b) => (b.stream?.viewers ?? -1) - (a.stream?.viewers ?? -1))
          .slice(0, FOLLOWED)
          .map(
            (c): Choice => ({
              key: `f${c.id}`,
              label: c.name,
              hint: c.stream ? (c.stream.game?.name ?? 'Live') : 'Followed',
              live: c.stream ? (c.stream.rerun ? 'rerun' : 'live') : null,
              icon: c.stream ? undefined : 'heart',
              run: () => void openChannel({ show: onopen, account: () => home.account }, c),
            }),
          )
      : [];
    const exact = followed.some((c) => c.label.toLowerCase() === q);
    const open: Choice[] =
      q && !exact
        ? [{ key: '+', label: `Open ${query.trim()}`, hint: 'New tab', icon: 'plus', run: () => void openTyped() }]
        : [];
    return [...tabChoices, ...followed, ...views, ...open];
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
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions, a11y_click_events_have_key_events -->
<div class="backdrop" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="switcher" role="dialog" aria-label="Quick switcher">
    <div class="search">
      <Icon name="search" size={16} />
      <input
        bind:this={field}
        bind:value={query}
        placeholder="Go to a tab, or open a channel"
        spellcheck="false"
        autocomplete="off"
        {onkeydown}
      />
    </div>
    {#if choices.length}
      <div class="list" role="listbox" aria-label="Results">
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
    z-index: var(--z-popover);
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
    box-shadow: 0 16px 48px var(--shadow);
    overflow: hidden;
  }
  .search {
    flex: none;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border-bottom: 1px solid var(--border);
    color: var(--muted);
  }
  input {
    flex: 1;
    height: 44px;
    background: none;
    border: 0;
    outline: none;
    font-size: 0.95rem;
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
    background: #ff3b3b;
    box-shadow: 0 0 6px 1px rgba(255, 59, 59, 0.75);
  }
  .dot.rerun {
    background: #f5b83d;
  }
  .empty {
    margin: 0;
    padding: 14px;
    color: var(--muted);
  }
</style>
