// Right-click and ⋯ menus. One shared menu per window, drawn by MenuHost. The rules every
// menu follows are in `menuItems.ts` (V101-30).

import { gate } from '../lib/gate.svelte';
import { type Align, point, type Rect } from '../lib/place';
import { type MenuEntry, type MenuItem, tidy } from './menuItems';
import { confirmer } from './menuConfirm';

export type { MenuEntry, MenuField, MenuItem } from './menuItems';

export const menu = $state({
  open: false,
  /** The pointer, or the button it opened from. */
  anchor: point(0, 0) as Rect,
  align: 'start' as Align,
  /** Lists shown so far; the last one is on screen. */
  stack: [] as { title: string; items: MenuEntry[] }[],
  /** V107-P5 The `confirm` item waiting for its second pick. */
  armed: null as MenuItem | null,
});

const confirming = confirmer<MenuItem>((item) => (menu.armed = item));

/** V107-P5 The pointer or keyboard went to `item`: an armed other item lets go. */
export function menuMoveTo(item: MenuItem): void {
  confirming.moveTo(item);
}

/** V107-P5 Escape: lets go of an armed item first; true when it did. */
export function menuDisarm(): boolean {
  return confirming.cancel();
}

/** At the pointer, or under a button (`align: 'end'` lines it up with the button's right edge). */
export function openMenu(
  at: { clientX: number; clientY: number } | Element,
  items: MenuEntry[],
  align: Align = 'start',
): void {
  menu.anchor = at instanceof Element ? at.getBoundingClientRect() : point(at.clientX, at.clientY);
  menu.align = align;
  confirming.cancel();
  menu.stack = [{ title: '', items: tidy(items) }];
  menu.open = true;
}

/** Under the button that was clicked, lined up with its right edge (⋯ buttons and pickers). */
export function openMenuAt(e: { currentTarget: EventTarget | null }, items: MenuEntry[]): void {
  if (e.currentTarget instanceof Element) openMenu(e.currentTarget, items, 'end');
}

export function closeMenu(): void {
  shut();
}

/** L1 Closed menus keep no items: their run closures hold the chat or player they came from. */
function shut(): void {
  confirming.cancel();
  menu.open = false;
  menu.stack = [];
  menu.armed = null;
}

export function pick(item: MenuItem): void {
  if (item.disabled) return;
  if (item.lock) {
    shut();
    if (item.unlock) item.unlock();
    else gate.ask(item.lock);
    return;
  }
  if (item.items) {
    confirming.cancel();
    menu.stack = [...menu.stack, { title: item.label, items: tidy(item.items) }];
    return;
  }
  if (!confirming.press(item, item.confirm)) return;
  shut();
  item.run?.();
}

export function back(): void {
  if (menu.stack.length < 2) return;
  confirming.cancel();
  menu.stack = menu.stack.slice(0, -1);
}
