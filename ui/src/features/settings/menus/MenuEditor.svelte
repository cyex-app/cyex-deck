<!-- V110-U67 One menu in Settings › Menus: a mock of how it opens, a toolbar to add rows, groups,
     lines and entries of your own, and who to preview it as. Every change saves at once. -->
<script lang="ts">
  import { tick, untrack } from 'svelte';

  import Button from '../../../components/Button.svelte';
  import Icon from '../../../components/Icon.svelte';
  import { type MenuEntry, openMenu } from '../../../components/menu.svelte';
  import { menuProblems } from '../../../components/menuItems';
  import Segmented from '../../../components/Segmented.svelte';
  import { placeholdersFor } from '../../../components/menus/custom.ts';
  import {
    addCustom,
    addGroup,
    addSep,
    addItem,
    editCustom,
    freshId,
    isDefault,
    moveInto,
    nudge,
    outOf,
    remove,
    rename,
    reset,
    resetPatch,
    savePatch,
    setHidden,
    startEditing,
    unused,
  } from '../../../components/menus/ops.ts';
  import { layoutOf, resolveMenu } from '../../../components/menus/resolve.ts';
  import type { CustomAction, MenuDef, Node } from '../../../components/menus/types.ts';
  import { settings } from '../../../lib/settings.svelte';
  import { toast } from '../../../lib/toast.svelte';
  import SectionHead from '../SectionHead.svelte';
  import CustomForm from './CustomForm.svelte';
  import { addPlace } from './menuDrag.ts';
  import MenuMock from './MenuMock.svelte';
  import { moveNote, type RowView, type SampleOf, WHO, type Who } from './model.ts';

  interface Props {
    def: MenuDef<unknown>;
    sample: SampleOf;
    onback: () => void;
  }

  let { def, sample, onback }: Props = $props();

  const stored = $derived(settings.v.menus.layouts[def.id]);

  let nodes = $state.raw<Node[]>(untrack(() => startEditing(def, stored)));
  let who = $state<Who>('viewer');
  let open = $state<string | null>(null);
  let focusKey = $state('');
  let renaming = $state<string | null>(null);
  /** The add/edit form: new, or the path of the custom entry being edited. */
  let form = $state<'new' | { path: number[] } | null>(null);
  let said = $state('');
  let mock = $state<{ focus(key: string): Promise<void> }>();

  // A change from elsewhere (another window, a backup restored) replaces what is drawn.
  $effect(() => {
    const fresh = startEditing(def, stored);
    untrack(() => {
      if (JSON.stringify(fresh) !== JSON.stringify(nodes)) nodes = fresh;
    });
  });

  const changed = $derived(!isDefault(def, nodes));
  const unusedItems = $derived(unused(def, nodes).sort((a, b) => a.name.localeCompare(b.name)));

  function save(next: Node[]) {
    void settings.patch('menus', savePatch(def, next) as never);
  }

  function commit(next: Node[], note: string, key?: string) {
    nodes = next;
    said = note;
    save(next);
    // The row's new home shows its flyout, so the move can be seen.
    if (key) {
      const home = next.find((n) => n.t === 'group' && n.children.some((c) => c.t !== 'sep' && c.id === key));
      if (home?.t === 'group') open = home.id;
      focusKey = key;
      void tick().then(() => mock?.focus(key));
    }
  }

  const labelOf = (row: RowView) => row.label || 'Line';

  // Rows

  function startRename(row: RowView) {
    if (row.kind === 'sep') return;
    if (!row.renamable) {
      said = row.slot ? `${row.label} is filled in by the app, it can’t be renamed` : `${row.label} changes with the situation, it can’t be renamed`;
      return;
    }
    focusKey = row.key;
    renaming = row.key;
  }

  function finishRename(row: RowView, label: string | null) {
    if (renaming !== row.key) return;
    renaming = null;
    if (label !== null && label.trim() !== row.label) {
      const next = rename(nodes, row.path, label);
      if (next !== nodes) commit(next, `Renamed ${row.label}`);
    }
    void tick().then(() => mock?.focus(row.key));
  }

  function toggle(row: RowView) {
    if (row.kind === 'sep') return;
    const next = setHidden(nodes, row.path, !row.hidden);
    commit(next, `${row.hidden ? 'Showing' : 'Hid'} ${row.label}`, row.key);
  }

  function drop(row: RowView) {
    // A built-in row only hides; the rest really goes, and Undo brings it back.
    if (row.kind === 'item') {
      if (!row.hidden) toggle(row);
      return;
    }
    const before = nodes;
    const next = remove(nodes, row.path);
    if (next === before) return;
    if (row.kind === 'group' && open === row.id) open = null;
    commit(next, `Removed ${labelOf(row)}`);
    toast.info(`Removed ${labelOf(row)}.`, {
      action: {
        label: 'Undo',
        run: () => {
          nodes = before;
          save(before);
        },
      },
    });
  }

  function move(row: RowView, dir: -1 | 1) {
    const next = nudge(nodes, row.path, dir);
    if (next === nodes) said = `${labelOf(row)} can’t move further`;
    else commit(next, moveNote(nodes, next, row.id, labelOf(row)), row.key);
  }

  function moreItems(row: RowView): MenuEntry[] {
    const groups = nodes.filter((n) => n.t === 'group');
    const here = row.path.length === 2 ? (nodes[row.path[0]] as { id: string }).id : null;
    const into: MenuEntry[] = [
      ...(here ? [{ label: 'Top level', icon: 'arrowUp' as const, run: () => commit(outOf(nodes, row.path), moveNote(nodes, outOf(nodes, row.path), row.id, row.label), row.key) }] : []),
      ...groups
        .filter((g) => g.id !== here)
        .map((g) => ({
          label: g.label,
          icon: 'list' as const,
          run: () => {
            const next = moveInto(nodes, row.path, g.id);
            commit(next, moveNote(nodes, next, row.id, labelOf(row)), row.key);
          },
        })),
    ];
    const custom = row.kind === 'custom';
    return [
      ...(row.kind === 'sep'
        ? []
        : [
            { label: 'Rename', icon: 'pencil' as const, hint: 'F2', disabled: !row.renamable, run: () => startRename(row) },
            ...(row.renamed ? [{ label: 'Use built-in name', icon: 'undo' as const, run: () => commit(rename(nodes, row.path, ''), `Name back to ${row.builtIn}`, row.key) }] : []),
            ...(custom ? [{ label: 'Edit', icon: 'settings' as const, run: () => (form = { path: row.path }) }] : []),
            { label: row.hidden ? 'Show' : 'Hide', icon: row.hidden ? ('eye' as const) : ('eyeOff' as const), hint: 'Space', run: () => toggle(row) },
          ]),
      'separator',
      { label: 'Move up', icon: 'arrowUp' as const, hint: 'Alt+↑', run: () => move(row, -1) },
      { label: 'Move down', icon: 'arrowDown' as const, hint: 'Alt+↓', run: () => move(row, 1) },
      ...(row.kind !== 'group' && into.length ? [{ label: 'Move into', icon: 'chevronRight' as const, items: into }] : []),
      ...(row.kind !== 'item'
        ? ['separator' as const, { label: row.kind === 'group' ? 'Remove group' : 'Remove', icon: 'trash' as const, danger: true, hint: 'Del', run: () => drop(row) }]
        : []),
    ];
  }

  function more(row: RowView, el: HTMLElement) {
    focusKey = row.key;
    openMenu(el, moreItems(row), 'end');
  }

  // Toolbar

  const place = () => addPlace(def, nodes, open);

  function addRows(el: Element) {
    const items: MenuEntry[] = unusedItems.map((it) => ({
      label: it.name,
      icon: it.icon,
      tag: it.when,
      run: () => commit(addItem(nodes, it.id, place()), `Added ${it.name}`, it.id),
    }));
    openMenu(el, items);
  }

  function newGroup() {
    const id = freshId(nodes, 'group');
    const next = addGroup(nodes, 'New group', addPlace(def, nodes, null));
    commit(next, 'Added a group', id);
    open = id;
    renaming = id;
  }

  function newLine() {
    const p = place();
    const next = addSep(nodes, p);
    commit(next, 'Added a line');
  }

  function submitForm(v: { label: string; action: CustomAction }) {
    if (form === 'new') {
      const id = freshId(nodes, 'custom');
      commit(addCustom(nodes, v, place()), `Added ${v.label}`, id);
    } else if (form) {
      const next = editCustom(nodes, form.path, v);
      const n = next[form.path[0]];
      commit(next, `Saved ${v.label}`, form.path.length === 1 && n.t !== 'sep' ? n.id : undefined);
    }
    form = null;
  }

  const editing = $derived.by(() => {
    if (!form || form === 'new') return undefined;
    const n = form.path.length === 1 ? nodes[form.path[0]] : (nodes[form.path[0]] as { children: Node[] }).children[form.path[1]];
    return n?.t === 'custom' ? { label: n.label, action: n.action } : undefined;
  });

  function resetAll() {
    const before = nodes;
    nodes = reset(def);
    open = null;
    renaming = null;
    form = null;
    said = 'Menu reset';
    void settings.patch('menus', resetPatch(def.id) as never);
    toast.info(`${def.label} menu reset.`, {
      action: {
        label: 'Undo',
        run: () => {
          nodes = before;
          save(before);
        },
      },
    });
  }

  // Soft notes: the house rules the menu follows, which a rearranging can break.
  const notes = $derived.by(() => {
    const ctx = sample(def.id, who);
    if (ctx === undefined) return [];
    try {
      const list = menuProblems(resolveMenu(def, layoutOf(def, nodes), ctx));
      return [...new Set(list.map((p) => (p.includes('icons on some') ? 'Some rows have icons and others don’t.' : p.includes('isn\'t last') ? 'A red row reads best last in its group.' : '')).filter(Boolean))];
    } catch {
      return [];
    }
  });

  const slotNames = $derived(def.items.filter((i) => i.slot).map((i) => i.name));
  const chips = $derived(placeholdersFor(def.id));
</script>

<div class="top">
  <button class="back" onclick={onback}><Icon name="chevronLeft" size={14} />All menus</button>
</div>
<SectionHead title={def.label} level={2} setting="menus:{def.id}">{def.where}.</SectionHead>

<div class="bar" role="toolbar" aria-label="Edit {def.label}">
  <div class="group">
    <Button small icon="plus" disabled={!unusedItems.length} onclick={(e) => addRows(e.currentTarget as Element)}
      >Add item</Button
    >
    <Button small icon="list" onclick={newGroup}>Add group</Button>
    <Button small icon="grip" onclick={newLine}>Add line</Button>
    <Button small icon="pencil" onclick={() => (form = form ? null : 'new')}>Add your own</Button>
  </div>
  <div class="group">
    <span class="lab">Preview as</span>
    <Segmented label="Preview as" options={WHO} value={who} onchange={(w) => (who = w)} neutral />
    <Button small variant="ghost" icon="undo" disabled={!changed} onclick={resetAll}>Reset</Button>
  </div>
</div>

{#if form}
  <CustomForm menuId={def.id} initial={editing} onsubmit={submitForm} oncancel={() => (form = null)} />
{/if}

<div class="stage">
  <MenuMock
    bind:this={mock}
    {def}
    {nodes}
    {who}
    {sample}
    bind:open
    bind:focusKey
    {renaming}
    oncommit={commit}
    say={(t) => (said = t)}
    onmore={more}
    onrename={finishRename}
    onstartrename={startRename}
    onremove={drop}
    ontoggle={toggle}
  />
  <ul class="help">
    <li>Drag a row to move it. Drop it on a group to put it inside, or pull it out of the flyout to the left.</li>
    <li>Double-click or F2 renames. Space hides, Delete removes. Alt with the arrows moves the selected row.</li>
    <li>Dim rows only show up in some cases ({who === 'viewer' ? 'switch the preview to see them' : 'the preview hides what does not apply'}).</li>
    {#if slotNames.length}<li>Dashed rows are filled in by the app: {slotNames.join(', ').toLowerCase()}. Move them, not rename.</li>{/if}
    {#if chips.length}<li>Your own entries can use {chips.map((c) => `$(${c})`).join(' ')}.</li>{/if}
  </ul>
</div>
{#each notes as n (n)}<p class="soft">{n}</p>{/each}

<span class="sr-only" role="status" aria-live="polite">{said}</span>

<style>
  .top {
    margin: 0 0 6px;
  }
  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    height: 26px;
    padding: 0 8px 0 4px;
    background: none;
    border: 0;
    border-radius: var(--radius);
    color: var(--muted);
    font-size: var(--text-sub);
  }
  .back:hover {
    color: var(--text);
    background: var(--surface-hover);
  }
  .bar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 8px 16px;
    margin: 4px 0 8px;
  }
  .group {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .lab {
    color: var(--muted);
    font-size: var(--text-hint);
  }
  .stage {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    gap: 16px 28px;
    margin-top: 10px;
  }
  .stage > :global(.mock) {
    flex: 0 1 620px;
  }
  .help {
    flex: 1 1 220px;
    max-width: 38ch;
    margin: 0;
    padding: 0 0 0 16px;
    color: var(--muted);
    font-size: var(--text-hint);
    line-height: 1.5;
  }
  .help li + li {
    margin-top: 6px;
  }
  .soft {
    margin: 10px 0 0;
    color: var(--yellow);
    font-size: var(--text-hint);
  }
</style>
