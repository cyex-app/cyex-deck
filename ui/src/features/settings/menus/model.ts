// V110-U67 What Settings › Menus draws for a menu: one view per row of the tree, read from
// the registry (the built-in name, icon and hint), the layout (own name, hidden) and a sample
// of who opens the menu (Viewer, Mod, Signed out). Pure, so node --test loads it.

import type { IconName } from '../../../components/icons';
import { KIND_ICONS, KIND_LABELS } from '../../../components/menus/custom.ts';
import { isDefault, type Path } from '../../../components/menus/ops.ts';
import { effectiveNodes } from '../../../components/menus/resolve.ts';
import type { ItemDef, MenuDef, MenuEntry, MenuLayout, Node } from '../../../components/menus/types.ts';
import { tidy } from '../../../components/menuItems.ts';

export type Who = 'viewer' | 'mod' | 'out';
export const WHO: { value: Who; label: string }[] = [
  { value: 'viewer', label: 'Viewer' },
  { value: 'mod', label: 'Mod' },
  { value: 'out', label: 'Signed out' },
];

/** The context a menu would be opened with for this kind of person, or undefined without a sample. */
export type SampleOf = (menuId: string, who: Who) => unknown;

export interface RowView {
  /** Stable while the row exists: its id, or its place for a line. */
  key: string;
  kind: Node['t'];
  path: Path;
  /** The node's id; empty for a line. */
  id: string;
  /** What the row shows now (own name, else the built-in one). */
  label: string;
  /** The built-in name, shown when the row has its own. */
  builtIn: string;
  icon: IconName | null;
  hint: string | null;
  danger: boolean;
  checked: boolean;
  /** Opens a second list: a group, or a built-in submenu. */
  sub: boolean;
  hidden: boolean;
  /** A small dim word, e.g. "mods". */
  tag: string | null;
  /** The row would not be in this menu for the person previewed. */
  absent: boolean;
  slot: boolean;
  renamable: boolean;
  renamed: boolean;
  /** Rows inside a group. */
  count: number;
  /** V110-B8 A moderator action, or a group of nothing else: only drawn when previewing as a Mod. */
  modOnly: boolean;
}

/** V110-B8 The tip under the Moderator section of the Add item list. */
export const MOD_HINT = 'Shown in chats you moderate.';

/** A moderator action, or a group holding only those (and lines). */
export function isModNode<C>(def: MenuDef<C>, node: Node): boolean {
  if (node.t === 'item') return def.items.find((i) => i.id === node.id)?.mod === true;
  if (node.t !== 'group') return false;
  const rows = node.children.filter((c) => c.t !== 'sep');
  return rows.length > 0 && rows.every((c) => isModNode(def, c));
}

const rowsOf = (built: MenuEntry | MenuEntry[] | null): MenuEntry[] => (built == null ? [] : Array.isArray(built) ? built : [built]);

/** What the registry item builds for the sample; null when it has none or its condition fails. */
function sampled<C>(def: MenuDef<C>, it: ItemDef<C>, who: Who, sample: SampleOf | undefined): MenuEntry[] | null {
  if (!sample) return null;
  const ctx = sample(def.id, who);
  if (ctx === undefined) return null;
  try {
    return rowsOf(it.build(ctx as C)).filter((e) => e !== 'separator');
  } catch {
    return null;
  }
}

export function rowView<C>(def: MenuDef<C>, node: Node, path: Path, who: Who, sample?: SampleOf): RowView {
  const base: RowView = {
    key: node.t === 'sep' ? `sep:${path.join('.')}` : node.id,
    kind: node.t,
    path,
    id: node.t === 'sep' ? '' : node.id,
    label: '',
    builtIn: '',
    icon: null,
    hint: null,
    danger: false,
    checked: false,
    sub: false,
    hidden: node.t !== 'sep' && node.hidden,
    tag: null,
    absent: false,
    slot: false,
    renamable: false,
    renamed: false,
    count: 0,
    modOnly: node.t !== 'sep' && isModNode(def, node),
  };
  if (node.t === 'sep') return base;
  if (node.t === 'group') {
    return { ...base, label: node.label, builtIn: node.label, icon: (node.icon as IconName | null) ?? null, sub: true, renamable: true, count: node.children.length };
  }
  if (node.t === 'custom') {
    return {
      ...base,
      label: node.label,
      builtIn: node.label,
      icon: (node.icon as IconName | null) ?? KIND_ICONS[node.action.kind],
      tag: KIND_LABELS[node.action.kind].toLowerCase(),
      renamable: true,
    };
  }
  const it = def.items.find((i) => i.id === node.id);
  if (!it) return { ...base, label: node.label ?? node.id, builtIn: node.id };
  const built = sampled(def, it, who, sample);
  const first = built?.[0];
  const shown = first && first !== 'separator' ? first : null;
  const slot = it.slot === true;
  const builtIn = shown?.label ?? it.name;
  const own = node.label?.trim() || null;
  return {
    ...base,
    label: own && it.renamable !== false && !slot ? own : builtIn,
    builtIn,
    icon: (shown?.icon as IconName | undefined) ?? it.icon ?? null,
    hint: shown?.hint ?? null,
    danger: shown?.danger === true,
    checked: shown?.checked === true,
    sub: !!shown?.items,
    tag: it.when ?? null,
    absent: !!sample && sample(def.id, who) !== undefined && !slot && (built?.length ?? 0) === 0,
    slot,
    renamable: it.renamable !== false && !slot,
    renamed: own !== null && it.renamable !== false && !slot && own !== builtIn,
  };
}

/** V110-B8 Moderator actions are drawn for a Mod preview only; the lines around them go with them. */
function visible(rows: RowView[], who: Who): RowView[] {
  if (who === 'mod') return rows;
  const out: RowView[] = [];
  for (const r of rows) {
    if (r.modOnly) continue;
    if (r.kind === 'sep' && (out.length === 0 || out.at(-1)!.kind === 'sep')) continue;
    out.push(r);
  }
  if (out.at(-1)?.kind === 'sep') out.pop();
  return out;
}

/** The rows of the menu itself. */
export function rootRows<C>(def: MenuDef<C>, nodes: Node[], who: Who, sample?: SampleOf): RowView[] {
  return visible(nodes.map((n, i) => rowView(def, n, [i], who, sample)), who);
}

/** The rows of a group's flyout. */
export function childRows<C>(def: MenuDef<C>, nodes: Node[], groupId: string | null, who: Who, sample?: SampleOf): RowView[] {
  const at = nodes.findIndex((n) => n.t === 'group' && n.id === groupId);
  const g = nodes[at];
  if (g?.t !== 'group') return [];
  return visible(g.children.map((n, i) => rowView(def, n, [at, i], who, sample)), who);
}

/** Tree order: each group is followed by its rows, whether or not its flyout is drawn open. */
export function treeOrder<C>(def: MenuDef<C>, nodes: Node[], open: string | null, who: Who, sample?: SampleOf): RowView[] {
  const out: RowView[] = [];
  for (const r of rootRows(def, nodes, who, sample)) {
    out.push(r);
    if (r.kind === 'group' && r.id === open) out.push(...childRows(def, nodes, r.id, who, sample));
  }
  return out;
}

/** True when the stored layout arranges the menu differently from the built-in one, for the "Changed" tag. */
export const isChanged = <C>(def: MenuDef<C>, stored: MenuLayout | undefined | null): boolean =>
  !!stored && !isDefault(def, effectiveNodes(def, stored));

/** What a screen reader hears after a move. */
export function moveNote(before: Node[], after: Node[], id: string, label: string): string {
  const where = (nodes: Node[]) => {
    for (let i = 0; i < nodes.length; i++) {
      const n = nodes[i];
      if (n.t !== 'sep' && n.id === id) return { group: null as string | null, at: i, of: nodes.length };
      if (n.t === 'group') {
        const j = n.children.findIndex((c) => c.t !== 'sep' && c.id === id);
        if (j >= 0) return { group: n.label, at: j, of: n.children.length };
      }
    }
    return null;
  };
  const was = where(before);
  const now = where(after);
  if (!now) return `Removed ${label}`;
  if (now.group && now.group !== was?.group) return `Moved ${label} into ${now.group}`;
  if (!now.group && was?.group) return `Moved ${label} out of ${was.group}`;
  return `Moved ${label} to ${now.at + 1} of ${now.of}`;
}

/**
 * V110-B8 The Add item list: viewer rows first, then a Moderator submenu with its hint, so a
 * moderator action never sits among the viewer ones. `pick` adds the item.
 */
export function addList<C>(items: ItemDef<C>[], pick: (it: ItemDef<C>) => void): MenuEntry[] {
  const entry = (it: ItemDef<C>): MenuEntry => ({ label: it.name, icon: it.icon, tag: it.when, run: () => pick(it) });
  const viewer = items.filter((i) => !i.mod).map(entry);
  const mod = items.filter((i) => i.mod).map(entry);
  if (!mod.length) return viewer;
  return tidy([
    ...viewer,
    'separator',
    { label: 'Moderator', icon: 'shield', note: MOD_HINT, items: [{ label: MOD_HINT, disabled: true }, 'separator', ...mod] },
  ]);
}
