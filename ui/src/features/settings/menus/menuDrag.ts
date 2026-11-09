// V110-U67 The pure part of dragging and typing in Settings › Menus: where a drop lands, what
// a key does to the selected row, where a new row goes. The pointer and the DOM are in
// MenuEditor; nothing here touches either, so node --test loads it.

import { indent, move, moveInto, nudge, outOf, type Path, type Place } from '../../../components/menus/ops.ts';
import type { MenuDef, Node } from '../../../components/menus/types.ts';

/** Pixels the pointer moves before a press turns into a drag (as for tabs). */
export const THRESHOLD = 5;

/** A drawn row, as the pointer sees it. */
export interface Box {
  key: string;
  /** The group whose flyout the row is in; null for the menu itself. */
  panel: string | null;
  /** Place in that panel's list. */
  index: number;
  kind: Node['t'];
  top: number;
  bottom: number;
}

/** A drawn panel: the menu itself (`id` null) or a group's flyout. */
export interface PanelBox {
  id: string | null;
  left: number;
  right: number;
  top: number;
  bottom: number;
}

export type Drop =
  /** Between rows: the line is drawn before or after `key`; an empty panel has no key. */
  | { t: 'line'; panel: string | null; index: number; key: string | null; edge: 'before' | 'after' }
  /** Onto a group row: the end of that group. */
  | { t: 'into'; group: string; key: string };

/** The middle share of a group row that means "into" and not "next to". */
const MIDDLE = 0.5;

/**
 * What a drop at (x, y) would do, or null outside both panels (and for a group over a flyout:
 * groups stay one level). `dragged` is the row's own key and kind.
 */
export function dropAt(panels: PanelBox[], boxes: Box[], x: number, y: number, dragged: { key: string; kind: Node['t'] }): Drop | null {
  const panel = panels.find((p) => x >= p.left && x <= p.right && y >= p.top - 8 && y <= p.bottom + 8);
  if (!panel) return null;
  if (panel.id !== null && dragged.kind === 'group') return null;
  const rows = boxes.filter((b) => b.panel === panel.id).sort((a, b) => a.index - b.index);
  if (!rows.length) return { t: 'line', panel: panel.id, index: 0, key: null, edge: 'before' };
  const at = rows.find((r) => y >= r.top && y <= r.bottom);
  if (!at) {
    if (y < rows[0].top) return { t: 'line', panel: panel.id, index: 0, key: rows[0].key, edge: 'before' };
    const last = rows[rows.length - 1];
    if (y > last.bottom) return { t: 'line', panel: panel.id, index: last.index + 1, key: last.key, edge: 'after' };
    // In a gap between two rows: the one below.
    const next = rows.find((r) => r.top > y)!;
    return { t: 'line', panel: panel.id, index: next.index, key: next.key, edge: 'before' };
  }
  const h = at.bottom - at.top;
  const f = h > 0 ? (y - at.top) / h : 0.5;
  if (at.kind === 'group' && panel.id === null && dragged.kind !== 'group' && at.key !== dragged.key && f >= (1 - MIDDLE) / 2 && f <= 1 - (1 - MIDDLE) / 2) {
    return { t: 'into', group: at.key, key: at.key };
  }
  return f < 0.5
    ? { t: 'line', panel: panel.id, index: at.index, key: at.key, edge: 'before' }
    : { t: 'line', panel: panel.id, index: at.index + 1, key: at.key, edge: 'after' };
}

/** The tree after dropping the row at `from`; the tree itself when the drop changes nothing. */
export function applyDrop(nodes: Node[], from: Path, drop: Drop): Node[] {
  if (drop.t === 'into') return moveInto(nodes, from, drop.group);
  const sameList = (from.length === 1 && drop.panel === null) || (from.length === 2 && nodes[from[0]]?.t === 'group' && (nodes[from[0]] as { id: string }).id === drop.panel);
  const own = from[from.length - 1];
  // The line's index counts the dragged row too; `move` counts the list without it.
  const index = sameList && own < drop.index ? drop.index - 1 : drop.index;
  const place: Place = { into: drop.panel, index };
  if (sameList && index === own) return nodes;
  return move(nodes, from, place);
}

/**
 * V110-F76 The group Add goes into: the open one only while the selected row is that group or
 * one of its rows. The first group's flyout is drawn open from the start, so "open" alone put
 * every new entry into it (into Moderate, which then showed up for viewers too).
 */
export function addGroupOf(nodes: Node[], openGroup: string | null, selected: string): string | null {
  if (!openGroup || !selected) return null;
  const at = nodes.findIndex((n) => n.t === 'group' && n.id === openGroup);
  const g = nodes[at];
  if (g?.t !== 'group') return null;
  if (selected === g.id || selected.startsWith(`sep:${at}.`)) return openGroup;
  return g.children.some((c) => c.t !== 'sep' && c.id === selected) ? openGroup : null;
}

/** Where Add puts a row: the end of the open group, else the top just before the first slot (your entries, plugins). */
export function addPlace<C>(def: MenuDef<C>, nodes: Node[], openGroup: string | null): Place {
  if (openGroup) {
    const g = nodes.find((n) => n.t === 'group' && n.id === openGroup);
    if (g?.t === 'group') return { into: openGroup, index: g.children.length };
  }
  const slots = new Set(def.items.filter((i) => i.slot).map((i) => i.id));
  const at = nodes.findIndex((n) => n.t === 'item' && slots.has(n.id));
  if (at >= 0) {
    // Keep the line that sets the slots apart where it is: land before it.
    const sep = at > 0 && nodes[at - 1].t === 'sep' ? at - 1 : at;
    return { into: null, index: sep };
  }
  return { into: null, index: nodes.length };
}

export type TreeAction =
  | { t: 'nudge'; dir: -1 | 1 }
  | { t: 'indent' }
  | { t: 'outdent' }
  | { t: 'remove' }
  | { t: 'rename' }
  | { t: 'toggle' }
  | { t: 'focus'; to: 'next' | 'prev' | 'first' | 'last' | 'child' | 'parent' };

export interface KeyLike {
  key: string;
  altKey?: boolean;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
}

/** What a key does to the row in focus, or null to leave the key alone. */
export function treeKey(e: KeyLike, row: { kind: Node['t']; depth: 1 | 2 }): TreeAction | null {
  if (e.ctrlKey || e.metaKey) return null;
  if (e.altKey) {
    if (e.key === 'ArrowUp') return { t: 'nudge', dir: -1 };
    if (e.key === 'ArrowDown') return { t: 'nudge', dir: 1 };
    if (e.key === 'ArrowRight') return row.depth === 1 && row.kind !== 'group' ? { t: 'indent' } : null;
    if (e.key === 'ArrowLeft') return row.depth === 2 ? { t: 'outdent' } : null;
    return null;
  }
  switch (e.key) {
    case 'ArrowDown':
      return { t: 'focus', to: 'next' };
    case 'ArrowUp':
      return { t: 'focus', to: 'prev' };
    case 'Home':
      return { t: 'focus', to: 'first' };
    case 'End':
      return { t: 'focus', to: 'last' };
    case 'ArrowRight':
      return row.kind === 'group' ? { t: 'focus', to: 'child' } : null;
    case 'ArrowLeft':
      return row.depth === 2 ? { t: 'focus', to: 'parent' } : null;
    case 'Enter':
      return row.kind === 'group' ? { t: 'focus', to: 'child' } : null;
    case 'F2':
      return row.kind === 'sep' ? null : { t: 'rename' };
    case ' ':
      return row.kind === 'sep' ? null : { t: 'toggle' };
    case 'Delete':
    case 'Backspace':
      return { t: 'remove' };
    default:
      return null;
  }
}

/** The tree after a move key; unchanged at the ends and where the move can't be made. */
export function applyMove(nodes: Node[], path: Path, a: TreeAction): Node[] {
  if (a.t === 'nudge') return nudge(nodes, path, a.dir);
  if (a.t === 'indent') return indent(nodes, path);
  if (a.t === 'outdent') return outOf(nodes, path);
  return nodes;
}
