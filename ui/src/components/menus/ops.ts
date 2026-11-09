// V110-U67 Edits to a menu's tree, for Settings › Menus. Pure: every function returns a new
// list and leaves its input alone. A node is addressed by its path, `[i]` at the top or
// `[group, i]` inside a group (lines have no id, so a path is the only way to name one).
// One level of groups: nothing goes into a group but plain rows.

import { effectiveNodes, idsOf, layoutOf } from './resolve.ts';
import type { CustomNode, GroupNode, ItemDef, MenuDef, MenuLayout, Node } from './types.ts';

export type Path = number[];

/** Where a node lands: in the group with this id (`null` is the top), at this index of the list once the node has left it. */
export interface Place {
  into: string | null;
  index: number;
}

export const MAX_LABEL = 40;

const clone = <T>(v: T): T => structuredClone(v);

/** The list a path points into, and the index in it. */
function locate(nodes: Node[], path: Path): { list: Node[]; i: number } | null {
  if (path.length === 1) return path[0] >= 0 && path[0] < nodes.length ? { list: nodes, i: path[0] } : null;
  if (path.length === 2) {
    const g = nodes[path[0]];
    if (g?.t !== 'group' || path[1] < 0 || path[1] >= g.children.length) return null;
    return { list: g.children, i: path[1] };
  }
  return null;
}

export function nodeAt(nodes: Node[], path: Path): Node | null {
  const at = locate(nodes, path);
  return at ? at.list[at.i] : null;
}

/** Path of the item, group or custom entry with this id. */
export function pathOf(nodes: Node[], id: string): Path | null {
  for (let i = 0; i < nodes.length; i++) {
    const n = nodes[i];
    if (n.t === 'sep') continue;
    if (n.id === id) return [i];
    if (n.t === 'group') {
      const j = n.children.findIndex((c) => c.t !== 'sep' && c.id === id);
      if (j >= 0) return [i, j];
    }
  }
  return null;
}

function listFor(nodes: Node[], into: string | null): Node[] | null {
  if (into === null) return nodes;
  const g = nodes.find((n): n is GroupNode => n.t === 'group' && n.id === into);
  return g ? g.children : null;
}

function insert(nodes: Node[], node: Node, place: Place | undefined): Node[] {
  const out = clone(nodes);
  const list = listFor(out, place?.into ?? null) ?? out;
  const at = place ? Math.max(0, Math.min(place.index, list.length)) : list.length;
  list.splice(at, 0, node);
  return out;
}

/** Cuts a node out of a copy of the tree. */
function cut(nodes: Node[], path: Path): { rest: Node[]; node: Node } | null {
  const rest = clone(nodes);
  const at = locate(rest, path);
  if (!at) return null;
  const [node] = at.list.splice(at.i, 1);
  return { rest, node };
}

/** Moves a node. Groups can't go into groups; a move that can't be made returns the tree as it was. */
export function move(nodes: Node[], from: Path, to: Place): Node[] {
  const c = cut(nodes, from);
  if (!c) return nodes;
  if (to.into !== null && c.node.t === 'group') return nodes;
  if (to.into !== null && !listFor(c.rest, to.into)) return nodes;
  return insert(c.rest, c.node, to);
}

/** Puts a node at the end of a group. */
export function moveInto(nodes: Node[], from: Path, groupId: string): Node[] {
  const g = nodes.find((n): n is GroupNode => n.t === 'group' && n.id === groupId);
  if (!g) return nodes;
  const sameGroup = from.length === 2 && nodes[from[0]] === g;
  return move(nodes, from, { into: groupId, index: g.children.length - (sameGroup ? 1 : 0) });
}

/** Takes a row out of its group, to the top just after that group. */
export function outOf(nodes: Node[], from: Path): Node[] {
  if (from.length !== 2) return nodes;
  return move(nodes, from, { into: null, index: from[0] + 1 });
}

/** Alt+Up and Alt+Down: one step within the list, no-op at the ends. */
export function nudge(nodes: Node[], from: Path, dir: -1 | 1): Node[] {
  const at = locate(nodes, from);
  if (!at) return nodes;
  const to = at.i + dir;
  if (to < 0 || to >= at.list.length) return nodes;
  const into = from.length === 2 ? (nodes[from[0]] as GroupNode).id : null;
  return move(nodes, from, { into, index: to });
}

/** Alt+Right: into the group right above, when there is one. */
export function indent(nodes: Node[], from: Path): Node[] {
  if (from.length !== 1 || from[0] === 0) return nodes;
  const above = nodes[from[0] - 1];
  return above.t === 'group' ? moveInto(nodes, from, above.id) : nodes;
}

function trimLabel(s: string): string {
  return [...s.trim()]
    .filter((c) => c >= ' ' && c !== '\u007f')
    .slice(0, MAX_LABEL)
    .join('')
    .trim();
}

/** Own name for a row. An item with an empty name goes back to its built-in one; a group or custom entry keeps its name. */
export function rename(nodes: Node[], path: Path, label: string): Node[] {
  const out = clone(nodes);
  const at = locate(out, path);
  const n = at?.list[at.i];
  if (!n || n.t === 'sep') return nodes;
  const next = trimLabel(label);
  if (n.t === 'item') n.label = next || null;
  else if (next) n.label = next;
  return out;
}

export function setHidden(nodes: Node[], path: Path, hidden: boolean): Node[] {
  const out = clone(nodes);
  const at = locate(out, path);
  const n = at?.list[at.i];
  if (!n || n.t === 'sep') return nodes;
  n.hidden = hidden;
  return out;
}

/** A free id like `group.1` or `custom.2`. */
export function freshId(nodes: Node[], prefix: string): string {
  const used = new Set(idsOf(nodes));
  for (let n = 1; ; n++) if (!used.has(`${prefix}.${n}`)) return `${prefix}.${n}`;
}

/** A registry item the tree doesn't hold, e.g. from the palette. Unchanged when it is already there. */
export function addItem(nodes: Node[], id: string, place?: Place): Node[] {
  if (idsOf(nodes).includes(id)) return nodes;
  return insert(nodes, { t: 'item', id, label: null, hidden: false }, place);
}

export function addGroup(nodes: Node[], label: string, place?: Place): Node[] {
  const name = trimLabel(label) || 'Group';
  const g: GroupNode = { t: 'group', id: freshId(nodes, 'group'), label: name, icon: null, hidden: false, children: [] };
  return insert(nodes, g, place?.into ? { into: null, index: place.index } : place);
}

export function addSep(nodes: Node[], place?: Place): Node[] {
  return insert(nodes, { t: 'sep' }, place);
}

export function addCustom(nodes: Node[], c: Pick<CustomNode, 'label' | 'action'> & { icon?: string | null }, place?: Place): Node[] {
  const label = trimLabel(c.label);
  if (!label) return nodes;
  const n: CustomNode = { t: 'custom', id: freshId(nodes, 'custom'), label, icon: c.icon ?? null, hidden: false, action: c.action };
  return insert(nodes, n, place);
}

/** Replaces a custom entry's name and action. */
export function editCustom(nodes: Node[], path: Path, c: Pick<CustomNode, 'label' | 'action'>): Node[] {
  const label = trimLabel(c.label);
  const out = clone(nodes);
  const at = locate(out, path);
  const n = at?.list[at.i];
  if (!label || n?.t !== 'custom') return nodes;
  n.label = label;
  n.action = c.action;
  return out;
}

/** Takes out a group (its rows go to the top where it stood), a line or a custom entry. Built-in items can only be hidden. */
export function remove(nodes: Node[], path: Path): Node[] {
  const out = clone(nodes);
  const at = locate(out, path);
  const n = at?.list[at.i];
  if (!at || !n || n.t === 'item') return nodes;
  if (n.t === 'group') at.list.splice(at.i, 1, ...n.children);
  else at.list.splice(at.i, 1);
  return out;
}

/** The built-in arrangement. */
export function reset<C>(def: MenuDef<C>): Node[] {
  return clone(def.defaults);
}

/** Registry items the tree shows no row for, for the palette. */
export function unused<C>(def: MenuDef<C>, nodes: Node[]): ItemDef<C>[] {
  const used = new Set(idsOf(nodes));
  return def.items.filter((i) => !used.has(i.id));
}

/** True when the tree is the built-in one, so nothing needs saving. */
export function isDefault<C>(def: MenuDef<C>, nodes: Node[]): boolean {
  return JSON.stringify(nodes) === JSON.stringify(def.defaults);
}

/** The tree to edit for a menu: the stored layout, brought up to date. */
export function startEditing<C>(def: MenuDef<C>, stored: MenuLayout | undefined | null): Node[] {
  return effectiveNodes(def, stored);
}

/** What to hand `settings.patch('menus', …)`: the layout, or `null` for the built-in tree so the key goes away. */
export function savePatch<C>(def: MenuDef<C>, nodes: Node[]): { layouts: Record<string, MenuLayout | null> } {
  return { layouts: { [def.id]: isDefault(def, nodes) ? null : layoutOf(def, nodes) } };
}

/** The patch that resets one menu. */
export function resetPatch(menuId: string): { layouts: Record<string, null> } {
  return { layouts: { [menuId]: null } };
}
