// Right-click and ⋯ menus. One shared menu per window, drawn by MenuHost. The rules every
// menu follows are in `menuItems.ts` (V101-30).

import { type Align, point, type Rect } from '../lib/place';
import { type MenuEntry, type MenuItem, tidy } from './menuItems';

export type { MenuEntry, MenuItem } from './menuItems';

export const menu = $state({
  open: false,
  /** The pointer, or the button it opened from. */
  anchor: point(0, 0) as Rect,
  align: 'start' as Align,
  /** Lists shown so far; the last one is on screen. */
  stack: [] as { title: string; items: MenuEntry[] }[],
});

/** At the pointer, or under a button (`align: 'end'` lines it up with the button's right edge). */
export function openMenu(
  at: { clientX: number; clientY: number } | Element,
  items: MenuEntry[],
  align: Align = 'start',
): void {
  menu.anchor = at instanceof Element ? at.getBoundingClientRect() : point(at.clientX, at.clientY);
  menu.align = align;
  menu.stack = [{ title: '', items: tidy(items) }];
  menu.open = true;
}

/** Under the button that was clicked, lined up with its right edge (⋯ buttons and pickers). */
export function openMenuAt(e: { currentTarget: EventTarget | null }, items: MenuEntry[]): void {
  if (e.currentTarget instanceof Element) openMenu(e.currentTarget, items, 'end');
}

export function closeMenu(): void {
  menu.open = false;
}

export function pick(item: MenuItem): void {
  if (item.disabled) return;
  if (item.items) {
    menu.stack = [...menu.stack, { title: item.label, items: tidy(item.items) }];
    return;
  }
  menu.open = false;
  item.run?.();
}

export function back(): void {
  if (menu.stack.length > 1) menu.stack = menu.stack.slice(0, -1);
}
