// V110-U67 Turns a menu def, the user's layout and the menu's context into the entries
// MenuHost shows. With no layout it is the built-in menu, unchanged.

import type { IconName } from '../icons';
import { type MenuEntry, tidy } from '../menuItems.ts';
import type { CustomNode, GroupNode, ItemDef, MenuDef, MenuLayout, Node } from './types.ts';

export interface ResolveOptions<C> {
  /** Builds a custom entry (see custom.ts `customEntry`); `null` leaves it out. Without it custom entries don't show. */
  custom?: (node: CustomNode, ctx: C) => MenuEntry | null;
}

// V110-U67 JSON, not structuredClone: a stored layout arrives as a settings-store proxy, which structuredClone refuses.
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v));

export function idsOf(nodes: Node[]): string[] {
  const out: string[] = [];
  for (const n of nodes) {
    if (n.t === 'sep') continue;
    out.push(n.id);
    if (n.t === 'group') out.push(...idsOf(n.children));
  }
  return out;
}

/** Item ids in the order the built-in layout shows them. */
function defaultOrder(nodes: Node[]): string[] {
  const out: string[] = [];
  for (const n of nodes) {
    if (n.t === 'item') out.push(n.id);
    else if (n.t === 'group') out.push(...defaultOrder(n.children));
  }
  return out;
}

/** The group of the built-in layout an item sits in, if any. */
function defaultGroup(nodes: Node[], id: string): string | null {
  for (const n of nodes) {
    if (n.t === 'group' && n.children.some((c) => c.t === 'item' && c.id === id)) return n.id;
  }
  return null;
}

/** The children of the built-in group with this id. */
function groupChildren(nodes: Node[], id: string): Node[] {
  return nodes.find((n): n is GroupNode => n.t === 'group' && n.id === id)?.children ?? [];
}

/** Where a list holds `id` (as an item): its index, else -1. */
function indexOfItem(list: Node[], id: string): number {
  return list.findIndex((n) => n.t === 'item' && n.id === id);
}

function insertAfter(nodes: Node[], afterId: string, add: Node): boolean {
  const at = indexOfItem(nodes, afterId);
  if (at >= 0) {
    nodes.splice(at + 1, 0, add);
    return true;
  }
  for (const n of nodes) {
    if (n.t === 'group' && insertAfter(n.children, afterId, add)) return true;
  }
  return false;
}

/**
 * The arrangement to show or edit: the user's tree without items that no longer exist, plus
 * the built-in items the layout has not seen yet. A new item goes after the item before it in
 * the built-in layout; else into its built-in group when that group is still there; else
 * after the item before it overall; else before the slots (your entries, plugins); else at the end. Items that are not in the
 * built-in layout (the palette's extras) are never placed. No layout: the built-in one.
 */
export function effectiveNodes<C>(def: MenuDef<C>, layout: MenuLayout | undefined | null): Node[] {
  if (!layout) return clone(def.defaults);
  const byId = new Map(def.items.map((i) => [i.id, i]));
  const keep = (list: Node[]): Node[] =>
    list
      .filter((n) => n.t !== 'item' || byId.has(n.id))
      .map((n) => (n.t === 'group' ? { ...n, children: keep(n.children) } : n));
  const tree = keep(clone(layout.nodes));

  const placed = new Set(idsOf(tree));
  const known = new Set(layout.known);
  const order = defaultOrder(def.defaults);
  const isSlot = (n: Node) => n.t === 'item' && byId.get(n.id)?.slot === true;
  order.forEach((id, at) => {
    if (placed.has(id) || known.has(id) || !byId.has(id)) return;
    const add: Node = { t: 'item', id, label: null, hidden: false };
    placed.add(id);
    const group = defaultGroup(def.defaults, id);
    // 1 After the item before it in its own built-in group (or the top level), wherever the user put that.
    const mates = group ? defaultOrder(groupChildren(def.defaults, group)) : defaultOrder(def.defaults.filter((n) => n.t !== 'group'));
    const before = mates.slice(0, mates.indexOf(id)).reverse().find((p) => placed.has(p));
    if (before && insertAfter(tree, before, add)) return;
    // 2 Into its built-in group, when the user kept it.
    const home = group && tree.find((n): n is GroupNode => n.t === 'group' && n.id === group);
    if (home) {
      home.children.push(add);
      return;
    }
    // 3 At the top, after whatever came before it overall.
    const prev = order.slice(0, at).reverse().find((p) => placed.has(p));
    const top = prev ? tree.findIndex((n) => (n.t === 'item' ? n.id === prev : n.t === 'group' && idsOf(n.children).includes(prev))) : -1;
    if (top >= 0) {
      tree.splice(top + 1, 0, add);
      return;
    }
    // 4 Before the slots, else last.
    const slot = tree.findIndex(isSlot);
    if (slot >= 0) tree.splice(slot, 0, add);
    else tree.push(add);
  });
  return tree;
}

/** What the layout would be saved as: the tree and every item the registry has now. */
export function layoutOf<C>(def: MenuDef<C>, nodes: Node[]): MenuLayout {
  return { nodes: clone(nodes), known: def.items.map((i) => i.id) };
}

const entriesOf = (built: MenuEntry | MenuEntry[] | null): MenuEntry[] => (built == null ? [] : Array.isArray(built) ? built : [built]);

function branch<C>(
  list: Node[],
  byId: Map<string, ItemDef<C>>,
  ctx: C,
  opts: ResolveOptions<C>,
): MenuEntry[] {
  const out: MenuEntry[] = [];
  for (const n of list) {
    if (n.t === 'sep') {
      out.push('separator');
    } else if (n.t === 'item') {
      const def = byId.get(n.id);
      if (!def || n.hidden) continue;
      const rows = entriesOf(def.build(ctx));
      const label = n.label?.trim();
      const rename = label && def.renamable !== false && !def.slot && rows.length === 1 && rows[0] !== 'separator';
      for (const r of rows) {
        if (r === 'separator') out.push(r);
        else out.push({ ...r, id: r.id ?? def.id, ...(rename ? { label } : {}) });
      }
    } else if (n.t === 'group') {
      if (n.hidden) continue;
      const items = tidy(branch(n.children, byId, ctx, opts));
      const rows = items.filter((e) => e !== 'separator');
      if (!rows.length) continue;
      // V110-N4 A group left with one visible row shows that row itself, not a submenu of one.
      if (rows.length === 1) {
        out.push(rows[0]);
        continue;
      }
      out.push({ id: n.id, label: n.label, ...(n.icon ? { icon: n.icon as IconName } : {}), items });
    } else {
      if (n.hidden || !opts.custom) continue;
      const row = opts.custom(n, ctx);
      if (row && row !== 'separator') out.push({ ...row, id: row.id ?? n.id });
    }
  }
  return out;
}

/** The entries to open a menu with. A condition that fails drops its row, an empty group drops out. */
export function resolveMenu<C>(
  def: MenuDef<C>,
  layout: MenuLayout | undefined | null,
  ctx: C,
  opts: ResolveOptions<C> = {},
): MenuEntry[] {
  const byId = new Map(def.items.map((i) => [i.id, i]));
  return tidy(branch(effectiveNodes(def, layout), byId, ctx, opts));
}
