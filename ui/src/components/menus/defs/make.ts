// V110-U66 Small helpers shared by the menu defs. Pure.

import type { IconName } from '../../icons';
import type { MenuEntry } from '../../menuItems.ts';
import type { ItemDef } from '../types.ts';

/** Who a preview (and a test) pretends to be. */
export type Who = 'viewer' | 'mod' | 'out';

export const noop = (): void => {};

/** An item whose row is built from the menu's context; `null` leaves it out. */
export function row<C>(
  id: string,
  name: string,
  icon: IconName,
  build: (c: C) => MenuEntry | MenuEntry[] | null,
  more: Partial<Omit<ItemDef<C>, 'id' | 'name' | 'icon' | 'build'>> = {},
): ItemDef<C> {
  return { id, name, icon, build, ...more };
}

/** A place other code fills in: your entries, plugins' entries, 7TV edits. */
export function slot<C>(id: string, name: string, build: (c: C) => MenuEntry[]): ItemDef<C> {
  return { id, name, slot: true, build: (c) => build(c) };
}

/** `build` of an item that works on a part of a bigger context. */
export function lift<C, P>(def: ItemDef<P>, pick: (c: C) => P | null | undefined, id = def.id): ItemDef<C> {
  return {
    ...def,
    id,
    build: (c) => {
      const p = pick(c);
      return p == null ? null : def.build(p);
    },
  };
}
