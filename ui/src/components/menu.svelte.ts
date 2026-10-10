// Right-click menus. One shared menu per window, drawn by MenuHost.

import { type Align, point, type Rect } from '../lib/place';
import type { IconName } from './icons';

export interface MenuItem {
  label: string;
  icon?: IconName;
  /** A color dot instead of an icon, e.g. a tab group's color. */
  swatch?: string;
  hint?: string;
  checked?: boolean;
  danger?: boolean;
  disabled?: boolean;
  run?: () => void;
  /** Opens a second list in place of this one. */
  items?: MenuEntry[];
}

export type MenuEntry = MenuItem | 'separator';

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
  menu.stack = [{ title: '', items }];
  menu.open = true;
}

export function closeMenu(): void {
  menu.open = false;
}

export function pick(item: MenuItem): void {
  if (item.disabled) return;
  if (item.items) {
    menu.stack = [...menu.stack, { title: item.label, items: item.items }];
    return;
  }
  menu.open = false;
  item.run?.();
}

export function back(): void {
  if (menu.stack.length > 1) menu.stack = menu.stack.slice(0, -1);
}
