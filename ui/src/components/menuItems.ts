// V101-30 One structure for every menu in the app (right-click and ⋯ buttons):
// - Order: the main action, other actions, going somewhere, copy, then switches; your own
//   entries and plugins after a line. Separators split the groups.
// - Red only for what can't be undone (delete, ban, block), and last in its group.
// - `hint` is a shortcut or the value a submenu has now, never an explanation; those go in
//   `note`, shown as a tooltip.
// - Action menus give every item an icon. Pick lists (sort, account, quality) and the text
//   menu (cut, copy, paste) have none; checks show what's picked.
// - Labels: a verb first where there is one, sentence case, no full stop.
// - A menu from a button opens under it; a right-click menu opens at the pointer.

import type { IconName } from './icons';

export interface MenuItem {
  label: string;
  icon?: IconName;
  /** A color dot instead of an icon, e.g. a tab group's color. */
  swatch?: string;
  /** A picture instead of an icon, e.g. an account. */
  avatar?: string;
  /** A shortcut, or the value a submenu has now. */
  hint?: string;
  /** A short explanation, shown on hover. */
  note?: string;
  checked?: boolean;
  danger?: boolean;
  /** A warning sign at the end, e.g. an account that needs signing in again. */
  warn?: boolean;
  disabled?: boolean;
  run?: () => void;
  /** Opens a second list in place of this one. */
  items?: MenuEntry[];
}

export type MenuEntry = MenuItem | 'separator';

/** Drops separators at either end and doubled ones, left by parts that didn't show. */
export function tidy(list: MenuEntry[]): MenuEntry[] {
  const out: MenuEntry[] = [];
  for (const e of list) {
    if (e === 'separator' && (out.length === 0 || out.at(-1) === 'separator')) continue;
    out.push(e);
  }
  if (out.at(-1) === 'separator') out.pop();
  return out;
}

const MAX_HINT = 14;

/** Where a list breaks the rules above; empty when it doesn't. For tests. */
export function menuProblems(list: MenuEntry[]): string[] {
  const problems: string[] = [];
  const items = list.filter((e): e is MenuItem => e !== 'separator');
  const pictured = items.filter((e) => e.icon || e.swatch || e.avatar).length;
  if (pictured && pictured !== items.length) problems.push('icons on some items only');
  list.forEach((e, i) => {
    if (e === 'separator') {
      if (i === 0 || i === list.length - 1 || list[i - 1] === 'separator') problems.push(`stray separator at ${i}`);
      return;
    }
    if (!/^[A-Z0-9#@]/.test(e.label) || e.label.endsWith('.')) problems.push(`label "${e.label}"`);
    if (e.hint && e.hint.length > MAX_HINT) problems.push(`hint "${e.hint}" explains; use note`);
    const next = list[i + 1];
    if (e.danger && next && next !== 'separator' && !next.danger) problems.push(`"${e.label}" isn't last in its group`);
    if (e.items) problems.push(...menuProblems(e.items).map((p) => `${e.label} › ${p}`));
  });
  return problems;
}
