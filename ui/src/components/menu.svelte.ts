// Right-click and ⋯ menus. One shared menu per window, drawn by MenuHost. The rules every
// menu follows are in `menuItems.ts` (V101-30).

import { gate } from '../lib/gate.svelte';
import { type Align, point, type Rect } from '../lib/place';
import { type MenuEntry, type MenuItem, tidy } from './menuItems';
import { confirmer } from './menuConfirm';
import { menuTrigger } from './menuTrigger';

export type { MenuEntry, MenuField, MenuItem } from './menuItems';

export const menu = $state({
  open: false,
  /** The pointer, or the button it opened from. */
  anchor: point(0, 0) as Rect,
  align: 'start' as Align,
  /** Lists shown so far. With `flyout` all are on screen side by side (V110-U69); otherwise the
   * last one replaces the others and has a back row. `from` is the row of the parent list that
   * opened it; `anchor` is where its flyout sits (the parent's box at that row). */
  stack: [] as { title: string; items: MenuEntry[]; from: number; anchor: Rect | null }[],
  /** V110-U69 Submenus open beside their row. False in a window too narrow for two menus. */
  flyout: false,
  /** V107-P5 The `confirm` item waiting for its second pick. */
  armed: null as MenuItem | null,
});

/** V110-U69 Narrower windows (pop-out chats) have no room for two menus: submenus swap in place. */
const FLYOUT_MIN_WIDTH = 520;

/** V109-F61 The button the open menu came from: clicking it again closes the menu. */
const trigger = menuTrigger();

/** The mouse-down is on the open menu's own button: the click that follows closes the menu. */
export function menuOwns(node: EventTarget | null): boolean {
  return trigger.owns(node);
}

/** V109-F84 The button the last menu opened from; closing the menu hands it the focus if nothing else has it. */
let opener: Element | null = null;
export function menuOpener(): HTMLElement | null {
  return opener instanceof HTMLElement && opener.isConnected ? opener : null;
}

const confirming = confirmer<MenuItem>((item) => (menu.armed = item));

/** V107-P5 The pointer or keyboard went to `item`: an armed other item lets go. */
export function menuMoveTo(item: MenuItem): void {
  confirming.moveTo(item);
}

/** V108-C21 The pointer is on (true) or left (false) the armed item's bubble: the 3 s waits. */
export function menuHold(on: boolean): void {
  confirming.hold(on);
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
  if (at instanceof Element && menu.open && trigger.isOpenFor(at)) {
    shut();
    return;
  }
  trigger.set(at instanceof Element ? at : null);
  opener = at instanceof Element ? at : null;
  menu.anchor = at instanceof Element ? at.getBoundingClientRect() : point(at.clientX, at.clientY);
  menu.align = align;
  confirming.cancel();
  menu.stack = [{ title: '', items: tidy(items), from: -1, anchor: null }];
  menu.flyout = typeof innerWidth === 'number' && innerWidth >= FLYOUT_MIN_WIDTH;
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
  trigger.clear();
  confirming.cancel();
  menu.open = false;
  menu.stack = [];
  menu.armed = null;
}

export function pick(item: MenuItem, viaKey = false): void {
  if (item.disabled) return;
  if (item.lock) {
    shut();
    if (item.unlock) item.unlock();
    else gate.ask(item.lock);
    return;
  }
  if (item.items) {
    confirming.cancel();
    menu.stack = [...menu.stack, { title: item.label, items: tidy(item.items), from: -1, anchor: null }];
    return;
  }
  if (!confirming.press(item, item.confirm, viaKey)) return;
  shut();
  item.run?.();
}

/** V110-U69 Opens row `index` of list `depth` as a flyout at `anchor`, replacing any deeper list. */
export function showSub(depth: number, index: number, anchor: Rect): void {
  const parent = menu.stack[depth];
  const item = parent?.items[index];
  if (!item || item === 'separator' || !item.items || item.disabled || item.lock) return;
  const next = menu.stack[depth + 1];
  if (next && next.from === index) return;
  trim(depth);
  menu.stack = [...menu.stack, { title: item.label, items: tidy(item.items), from: index, anchor }];
}

/** V110-U69 Closes every list deeper than `depth`. */
export function trim(depth: number): void {
  if (menu.stack.length <= depth + 1) return;
  confirming.cancel();
  menu.stack = menu.stack.slice(0, depth + 1);
}

export function back(): void {
  if (menu.stack.length < 2) return;
  confirming.cancel();
  menu.stack = menu.stack.slice(0, -1);
}
