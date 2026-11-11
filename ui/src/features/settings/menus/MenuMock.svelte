<!-- V110-U67 A menu drawn as it opens: the panel, and the selected group's flyout beside it.
     Rows drag (pointer events, 5 px before it starts) and the whole thing is a keyboard tree:
     arrows move the focus, Alt+arrows move the row, F2 renames, Space hides, Delete removes. -->
<script lang="ts">
  import { tick } from 'svelte';

  import '../../../components/menu.css';
  import type { MenuDef, Node } from '../../../components/menus/types.ts';
  import { applyDrop, applyMove, type Box, type Drop, dropAt, type PanelBox, THRESHOLD, treeKey } from './menuDrag.ts';
  import MockRow from './MockRow.svelte';
  import { childRows, moveNote, rootRows, type RowView, type SampleOf, treeOrder, type Who } from './model.ts';

  interface Props {
    def: MenuDef<unknown>;
    nodes: Node[];
    who: Who;
    sample: SampleOf;
    /** The group whose flyout is drawn. */
    open: string | null;
    focusKey: string;
    renaming: string | null;
    /** A change of the tree; `note` is what a screen reader hears. */
    oncommit: (next: Node[], note: string, key?: string) => void;
    say: (note: string) => void;
    onmore: (row: RowView, el: HTMLElement) => void;
    onrename: (row: RowView, label: string | null) => void;
    onstartrename: (row: RowView) => void;
    onremove: (row: RowView) => void;
    ontoggle: (row: RowView) => void;
  }

  let {
    def,
    nodes,
    who,
    sample,
    open = $bindable(),
    focusKey = $bindable(),
    renaming,
    oncommit,
    say,
    onmore,
    onrename,
    onstartrename,
    onremove,
    ontoggle,
  }: Props = $props();

  const roots = $derived(rootRows(def, nodes, who, sample));
  const order = $derived(treeOrder(def, nodes, open, who, sample));
  const openGroup = $derived(roots.find((r) => r.kind === 'group' && r.id === open) ?? null);
  const kids = $derived(openGroup ? childRows(def, nodes, openGroup.id, who, sample) : []);
  const stopKey = $derived(order.some((r) => r.key === focusKey) ? focusKey : (order[0]?.key ?? ''));

  let tree = $state<HTMLElement>();
  let rootEl = $state<HTMLElement>();
  let flyTop = $state(0);

  // The flyout starts level with its group's row.
  $effect(() => {
    void [open, nodes, who, kids.length];
    void tick().then(() => {
      const row = open ? rootEl?.querySelector<HTMLElement>(`[data-key="${CSS.escape(open)}"]`) : null;
      flyTop = row ? Math.max(0, row.offsetTop - 4) : 0;
    });
  });

  function rowEl(key: string): HTMLElement | null {
    return tree?.querySelector<HTMLElement>(`[data-key="${CSS.escape(key)}"]`) ?? null;
  }

  async function focusRow(key: string) {
    focusKey = key;
    await tick();
    rowEl(key)?.focus();
  }

  /** Puts the cursor on a row (after a move or an add). */
  export function focus(key: string): Promise<void> {
    return focusRow(key);
  }

  function select(row: RowView) {
    focusKey = row.key;
    if (row.kind === 'group') open = row.id;
    else if (row.path.length === 2) open = (nodes[row.path[0]] as { id: string }).id;
  }

  // Keys

  function onkeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    if (target.closest('input')) return;
    const el = target.closest<HTMLElement>('[role="treeitem"]');
    const row = el && order.find((r) => r.key === el.dataset.key);
    if (!el || !row) return;
    if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      onmore(row, el);
      return;
    }
    const a = treeKey(e, { kind: row.kind, depth: row.path.length as 1 | 2 });
    if (!a) return;
    e.preventDefault();
    const at = order.indexOf(row);
    switch (a.t) {
      case 'focus': {
        let to: RowView | undefined;
        if (a.to === 'next') to = order[at + 1];
        else if (a.to === 'prev') to = order[at - 1];
        else if (a.to === 'first') to = order[0];
        else if (a.to === 'last') to = order[order.length - 1];
        else if (a.to === 'parent') to = roots[row.path[0]];
        else if (row.kind === 'group') {
          open = row.id;
          const first = childRows(def, nodes, row.id, who, sample)[0];
          if (first) to = first;
          else say(`${row.label} is empty`);
        }
        if (to) {
          if (to.kind === 'group') open = to.id;
          void focusRow(to.key);
        }
        return;
      }
      case 'rename':
        return onstartrename(row);
      case 'toggle':
        return ontoggle(row);
      case 'remove':
        return onremove(row);
      default: {
        const next = applyMove(nodes, row.path, a);
        if (next === nodes) return say(`${row.label} can’t move further`);
        oncommit(next, moveNote(nodes, next, row.id, row.label), row.key);
      }
    }
  }

  // Dragging

  let drag = $state<{ row: RowView; x: number; y: number; drop: Drop | null } | null>(null);

  function measure(): { panels: PanelBox[]; boxes: Box[] } {
    const panels: PanelBox[] = [];
    const boxes: Box[] = [];
    for (const p of tree?.querySelectorAll<HTMLElement>('[data-panel]') ?? []) {
      const r = p.getBoundingClientRect();
      const id = p.dataset.panel || null;
      panels.push({ id, left: r.left, right: r.right, top: r.top, bottom: r.bottom });
      for (const el of p.querySelectorAll<HTMLElement>('[data-key]')) {
        const b = el.getBoundingClientRect();
        boxes.push({ key: el.dataset.key!, panel: id, index: Number(el.dataset.index), kind: el.dataset.kind as Node['t'], top: b.top, bottom: b.bottom });
      }
    }
    return { panels, boxes };
  }

  let swallow = false;
  /** A2-10 Ends a drag in flight without dropping; run on unmount. */
  let stopDrag: (() => void) | null = null;
  $effect(() => () => stopDrag?.());

  function press(e: PointerEvent, row: RowView) {
    if (e.button !== 0 || renaming) return;
    const el = e.currentTarget as HTMLElement;
    const id = e.pointerId;
    const from = { x: e.clientX, y: e.clientY };
    let began = false;

    const move = (ev: PointerEvent) => {
      if (ev.pointerId !== id) return;
      if (!began) {
        if (Math.hypot(ev.clientX - from.x, ev.clientY - from.y) < THRESHOLD) return;
        began = true;
        try {
          el.setPointerCapture(id);
        } catch {
          /* the row went away: the window listeners still follow */
        }
        document.body.classList.add('dragging');
        focusKey = row.key;
      }
      const { panels, boxes } = measure();
      drag = { row, x: ev.clientX, y: ev.clientY, drop: dropAt(panels, boxes, ev.clientX, ev.clientY, { key: row.key, kind: row.kind }) };
    };
    const finish = (ok: boolean) => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', up);
      removeEventListener('pointercancel', cancel);
      removeEventListener('keydown', esc, true);
      // A2-10 Also gone when the window loses focus mid-drag.
      removeEventListener('blur', blur);
      stopDrag = null;
      document.body.classList.remove('dragging');
      if (!began) return;
      swallow = true;
      setTimeout(() => (swallow = false));
      const drop = drag?.drop;
      drag = null;
      if (!ok || !drop) return;
      const next = applyDrop(nodes, row.path, drop);
      if (next === nodes) return;
      if (drop.t === 'into') open = drop.group;
      else if (drop.panel) open = drop.panel;
      oncommit(next, moveNote(nodes, next, row.id, row.label || 'Line'), row.key);
    };
    const blur = () => finish(false);
    stopDrag = blur;
    const up = (ev: PointerEvent) => ev.pointerId === id && finish(true);
    const cancel = (ev: PointerEvent) => ev.pointerId === id && finish(false);
    const esc = (ev: KeyboardEvent) => {
      if (ev.key !== 'Escape') return;
      ev.preventDefault();
      ev.stopPropagation();
      finish(false);
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', up);
    addEventListener('pointercancel', cancel);
    addEventListener('keydown', esc, true);
    addEventListener('blur', blur);
  }

  const lineOf = (row: RowView): 'before' | 'after' | null =>
    drag?.drop?.t === 'line' && drag.drop.key === row.key ? drag.drop.edge : null;
  const intoOf = (row: RowView): boolean => drag?.drop?.t === 'into' && drag.drop.key === row.key;
</script>

<div class="wrap">
<!-- svelte-ignore a11y_no_noninteractive_element_to_interactive_role (a tree of rows) -->
<div
  class="mock"
  role="tree"
  aria-label="{def.label} menu"
  tabindex="-1"
  bind:this={tree}
  {onkeydown}
  onclickcapture={(e) => {
    if (swallow) {
      e.stopPropagation();
      e.preventDefault();
    }
  }}
>
  <div class="cx-menu still panel" data-panel="" bind:this={rootEl} role="presentation">
    {#each roots as row (row.key)}
      <MockRow
        {row}
        depth={1}
        stop={row.key === stopKey}
        selected={row.key === focusKey}
        open={row.kind === 'group' && row.id === open}
        line={lineOf(row)}
        into={intoOf(row)}
        dragging={drag?.row.key === row.key}
        renaming={renaming === row.key}
        onpress={(e) => press(e, row)}
        onselect={() => select(row)}
        onstartrename={() => onstartrename(row)}
        onrename={(label) => onrename(row, label)}
        onmore={(el) => onmore(row, el)}
      />
    {/each}
    {#if !roots.length}<div class="empty">Nothing here. Add a row from the toolbar.</div>{/if}
  </div>
  {#if openGroup}
    <div class="cx-menu still panel fly" data-panel={openGroup.id} role="group" aria-label="{openGroup.label} rows" style:--fly-top="{flyTop}px">
      {#each kids as row (row.key)}
        <MockRow
          {row}
          depth={2}
          stop={row.key === stopKey}
          selected={row.key === focusKey}
          open={false}
          line={lineOf(row)}
          into={false}
          dragging={drag?.row.key === row.key}
          renaming={renaming === row.key}
          onpress={(e) => press(e, row)}
          onselect={() => select(row)}
          onstartrename={() => onstartrename(row)}
          onrename={(label) => onrename(row, label)}
          onmore={(el) => onmore(row, el)}
        />
      {/each}
      {#if !kids.length}
        <div class="empty slotline" class:target={drag?.drop?.t === 'line' && drag.drop.panel === openGroup.id}>Drop rows here</div>
      {/if}
    </div>
  {/if}
</div>
</div>

{#if drag}
  <div class="ghost cx-menu still" style:left="{drag.x + 14}px" style:top="{drag.y + 10}px" aria-hidden="true">
    <span class="cx-text">{drag.row.label || 'Line'}</span>
  </div>
{/if}

<style>
  /* The container: its own width decides whether the flyout sits beside the menu or under it. */
  .wrap {
    container-type: inline-size;
    min-width: 0;
  }
  .mock {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    min-width: 0;
  }
  .mock:focus {
    outline: none;
  }
  .panel {
    flex: 0 0 auto;
    width: min(100%, 290px);
    min-width: 0;
  }
  .fly {
    margin-top: var(--fly-top, 0);
  }
  /* A narrow page has no room beside it: the flyout goes under. */
  @container (max-width: 600px) {
    .mock {
      flex-direction: column;
      align-items: stretch;
    }
    .panel {
      width: 100%;
      max-width: 340px;
    }
    .fly {
      margin: 0 0 0 20px;
    }
  }
  .empty {
    display: flex;
    align-items: center;
    height: 30px;
    padding: 0 9px;
    color: var(--dim);
    font-size: var(--text-hint);
  }
  .slotline {
    margin: 2px 0;
    height: 28px;
    border: 1px dashed var(--border-strong);
    border-radius: var(--radius);
  }
  .slotline.target {
    border-color: var(--accent);
    background: var(--accent-dim);
    color: var(--text);
  }
  .ghost.cx-menu {
    position: fixed;
    z-index: var(--z-tooltip);
    min-width: 0;
    max-width: 240px;
    height: 30px;
    display: flex;
    align-items: center;
    padding: 0 12px;
    pointer-events: none;
    opacity: 0.92;
  }
</style>
