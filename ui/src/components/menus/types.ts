// V110-U67 The menu registry's shapes. Each menu has a def (its items and its built-in
// layout); the user's layout (settings partition `menus`) rearranges, renames and hides.
// Pure types plus tiny helpers for writing defaults; no runes, so node --test loads it.

import type { CustomAction } from '../../ipc/bindings/CustomAction';
import type { MenuLayout } from '../../ipc/bindings/MenuLayout';
import type { MenuNode } from '../../ipc/bindings/MenuNode';
import type { IconName } from '../icons';
import type { MenuEntry } from '../menuItems';

export type { CustomAction, MenuEntry, MenuLayout, MenuNode };

/** A row of a layout: an item, a group (one level), a line or a custom entry. */
export type Node = MenuNode;
export type ItemNode = Extract<Node, { t: 'item' }>;
export type GroupNode = Extract<Node, { t: 'group' }>;
export type CustomNode = Extract<Node, { t: 'custom' }>;

/** Every menu that can be arranged. */
export type MenuId =
  | 'chat.name'
  | 'chat.message'
  | 'chat.emote'
  | 'chat.more'
  | 'feed.name'
  | 'feed.message'
  | 'tabs.tab'
  | 'tabs.tile'
  | 'tabs.bar'
  | 'tabs.group'
  | 'player'
  | 'browse.card'
  | 'whispers.thread'
  | 'clips.clip';

/** One row a menu can show. `C` is what the menu is opened with (the user, the message, the tab…). */
export interface ItemDef<C> {
  /** Stable across versions; stored in layouts. */
  id: string;
  /** What the manager's mock and palette call it. */
  name: string;
  icon?: IconName;
  /** The entry (or several, e.g. a run of rows), or `null` when its condition fails. */
  build(ctx: C): MenuEntry | MenuEntry[] | null;
  /** `false` for two-state labels (Play/Pause, Hide/Show): the user can't rename them. Default true. */
  renamable?: boolean;
  /** Words the palette search matches besides `name`. */
  tags?: string[];
  /** A place where other code fills in rows (your entries, plugins, async 7TV edits): never renamed. */
  slot?: true;
  /** V110-B8 A moderator action (ban, timeout, delete, pin…). It builds nothing for an account that may not moderate the chat, and the manager keeps it in its own Moderator group, out of a Viewer or Signed out preview. */
  mod?: true;
  /** Shown dim in the manager when it only appears sometimes, e.g. "mods", "signed in". */
  when?: string;
}

export interface MenuDef<C> {
  id: MenuId;
  /** The manager's name for the menu. */
  label: string;
  /** Where it opens, in the manager's words. */
  where: string;
  items: ItemDef<C>[];
  /** The built-in layout. Items missing here are only in the palette. */
  defaults: Node[];
}

/** Layouts as stored: by menu id. */
export type Layouts = Partial<Record<string, MenuLayout>>;

// Writing defaults.

export const item = (id: string): ItemNode => ({ t: 'item', id, label: null, hidden: false });
export const group = (id: string, label: string, children: Node[], icon?: IconName): GroupNode => ({
  t: 'group',
  id,
  label,
  icon: icon ?? null,
  hidden: false,
  children,
});
export const SEP: Node = { t: 'sep' };
