<!-- V105-13 The chat font: one A-Z list of the fonts this PC really has (bundled, added, installed), each in its own face. -->
<script lang="ts">
  import { tick } from 'svelte';

  import Icon from '../../components/Icon.svelte';
  import Popover from '../../components/Popover.svelte';
  import { tooltip } from '../../components/tooltip.svelte';
  import { draws } from '../../design/fontProbe';
  import type { FontChoice } from '../../ipc/bindings/FontChoice';
  import { fontsIpc } from '../../ipc/theme';
  import { RANK, rank } from '../chat/completion';
  import { scrollWithin } from '../../lib/scrollWithin';
  import SearchField from '../../components/SearchField.svelte';

  interface Props {
    /** Empty = the app font. */
    value: string;
    onpick: (name: string) => void;
    onremove: (name: string) => void;
  }

  let { value, onpick, onremove }: Props = $props();

  let open = $state(false);
  let query = $state('');
  let list = $state<FontChoice[]>([]);
  let active = $state(0);
  let failed = $state(false);
  let listEl = $state<HTMLUListElement>();

  // Installed fonts the webview can't draw by that name would fall back silently (fontProbe).
  const load = () =>
    fontsIpc
      .list()
      .then((l) => {
        failed = false;
        list = l.filter((f) => f.kind !== 'installed' || draws(f.name));
      })
      .catch(() => (failed = true));

  function remove(name: string) {
    onremove(name);
    list = list.filter((f) => f.name !== name);
  }

  $effect(() => {
    void load();
  });
  $effect(() => {
    if (!open) return;
    query = '';
    active = 0;
    void load();
  });

  const missing = $derived(!!value && list.length > 0 && !list.some((f) => f.name.toLowerCase() === value.toLowerCase()));
  const q = $derived(query.trim());
  /** Typed: best match first (V101-16 rank), A-Z within. An empty name is the app font. */
  const found = $derived.by(() => {
    const app = { name: '', kind: 'app' as const };
    if (!q) return [app, ...list];
    const hits = list
      .map((f) => ({ f, r: rank(f.name, q) }))
      .filter((h) => h.r.rank > RANK.none)
      .sort((a, b) => b.r.rank - a.r.rank || b.r.close - a.r.close);
    return rank('App font', q).rank > RANK.none ? [app, ...hits.map((h) => h.f)] : hits.map((h) => h.f);
  });

  /** Every row paints in its own font: the rest waits for a search. */
  const LIMIT = 150;
  const rows = $derived(found.slice(0, LIMIT));

  const selected = (name: string) => (name === '' ? !value : name.toLowerCase() === value.toLowerCase());
  const face = (name: string) => `"${name}"`;

  function pick(name: string) {
    onpick(name);
    open = false;
  }

  function onkey(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (rows.length) active = (active + (e.key === 'ArrowDown' ? 1 : -1) + rows.length) % rows.length;
      void tick().then(() => {
        const hot = listEl?.querySelector<HTMLElement>('li.hot');
        if (hot) scrollWithin(hot, 'nearest');
      });
    } else if (e.key === 'Enter' && rows[active]) {
      e.preventDefault();
      pick(rows[active].name);
    }
  }
</script>

<div class="picker">
<div class="wrap">
  <button class="field trigger" aria-haspopup="listbox" aria-expanded={open} onclick={() => (open = !open)}>
    <span class="name" style:font-family={value && !missing ? face(value) : 'var(--font-ui)'}>{value || 'App font'}</span>
    <Icon name="chevronDown" size={13} />
  </button>
  <Popover bind:open align="start" label="Chat font">
    <div class="panel">
      <SearchField autofocus placeholder="Search fonts" bind:value={query} oninput={() => (active = 0)} onkeydown={onkey} />
      <ul class="fp-list" role="listbox" bind:this={listEl} aria-label="Chat font">
        {#each rows as f, i (f.name)}
          <li role="option" aria-selected={selected(f.name)} class:hot={i === active}>
            <button class="row" class:on={selected(f.name)} onclick={() => pick(f.name)} onmousemove={() => (active = i)}>
              <span class="face" style:font-family={f.name ? face(f.name) : 'var(--font-ui)'}>{f.name || 'App font'}</span>
              {#if selected(f.name)}<Icon name="check" size={13} />{/if}
            </button>
            {#if f.kind === 'added'}
              <button class="remove" aria-label="Remove {f.name}" use:tooltip={'Remove'} onclick={() => remove(f.name)}
                ><Icon name="x" size={12} /></button
              >
            {/if}
          </li>
        {/each}
      </ul>
      {#if failed}<p class="empty">Couldn’t read the fonts.</p>{:else if !rows.length}<p class="empty">No font by that name.</p>{:else if found.length > rows.length}<p class="empty">{found.length - rows.length} more. Type to find one.</p>{/if}
    </div>
  </Popover>
</div>
{#if missing}<p class="note">Not on this PC, so chat uses the app font.</p>{/if}
</div>

<style>
  .wrap {
    position: relative;
  }
  .trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 240px;
    height: 30px;
    padding: 0 10px;
    text-align: left;
  }
  .trigger :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .panel {
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-width: 260px;
  }
  ul {
    max-height: 320px;
    margin: 0;
    padding: 0;
    overflow-y: auto;
    list-style: none;
  }
  li {
    position: relative;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    width: 100%;
    height: 28px;
    padding: 0 10px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: 0.9rem;
    text-align: left;
  }
  .row.on {
    color: var(--text);
  }
  li.hot .row,
  .row:hover {
    background: var(--surface-hover);
  }
  /* Not `.label`: that global class (tokens.css) is the small capitals of section titles. */
  .face {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .remove {
    position: absolute;
    top: 50%;
    right: 24px;
    display: grid;
    opacity: 0;
    place-items: center;
    width: 20px;
    height: 20px;
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--dim);
    transform: translateY(-50%);
  }
  /* Hidden until hover, but still focusable by Tab (display: none would skip it). */
  /* 24 px to hit, 20 px to see. */
  .remove::before {
    content: "";
    position: absolute;
    inset: -2px;
  }
  li:hover .remove,
  li:focus-within .remove {
    opacity: 1;
  }
  .remove:hover {
    background: var(--surface-hover);
    color: var(--text);
  }
  .empty {
    margin: 0;
    padding: 6px 10px;
    color: var(--muted);
    font-size: 0.85rem;
  }
  .note {
    margin: 4px 0 0;
    color: var(--yellow);
    font-size: 0.8rem;
  }
</style>
