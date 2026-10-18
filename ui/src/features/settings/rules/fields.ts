// Columns of a rule list editor (2.6, F2, F3, S4): which key, and how to edit it.

import type { IconName } from '../../../components/icons';

export interface Field<T> {
  key: keyof T & string;
  /** Checkbox text, tooltip and accessible name. */
  label: string;
  /** V105-18 A check shown as this icon in a split row; `label` and `hint` are its tooltip. */
  icon?: IconName;
  /** V105-17 A sound file: picked with a button, then played or cleared from its menu. */
  kind: 'text' | 'area' | 'check' | 'color' | 'number' | 'select' | 'list' | 'sound';
  placeholder?: string;
  /** V105-16 The tooltip under the label: what the field or option does. */
  hint?: string;
  options?: readonly { value: string; label: string }[];
  /** CSS width; text fields grow when unset. */
  width?: string;
  /** V105-18 The flexible field: what the rule looks for. */
  main?: boolean;
  /** V106-10 Split rows: a short field drawn before the main one, e.g. a command's name. */
  lead?: boolean;
  /** V105-18 What a match does (color, sound, …): the second group of a split row. */
  alert?: boolean;
  /** Monospace text, e.g. a regex. */
  mono?: boolean;
  /** C3 A number's name, shown beside it (the label stays the tooltip). */
  caption?: string;
  /** C3 After a number, e.g. "s". */
  unit?: string;
  /** C2 Own line under the row: a long field such as a response or a filter. */
  wide?: boolean;
  /** Only shown while this holds, e.g. the regex flavor. */
  when?: (item: T) => boolean;
}

/** `#rrggbbaa` from a picked `#rrggbb`, keeping the alpha it had (or 50%). */
export function withAlpha(picked: string, old: string): string {
  const alpha = old.length === 9 ? old.slice(7) : '7f';
  return `${picked}${alpha}`;
}

/** "a, b c" → ["a", "b", "c"]. */
export function splitList(text: string): string[] {
  return text
    .split(/[\s,]+/)
    .map((s) => s.trim())
    .filter(Boolean);
}

/** b-10 A row's edit applied to the row as it is now. `seen` is the row the edit started
 *  from; when two fields change in one tick, the second edit still starts from the row
 *  before the first, so only the keys it changed are taken. */
export function applyEdit<T extends Record<string, unknown>>(now: T, seen: T, next: T): T {
  const out: Record<string, unknown> = { ...now };
  for (const k of Object.keys(next)) if (next[k] !== seen[k]) out[k] = next[k];
  return out as T;
}

/** rb-2 Row `i`'s edit, made from `seen`, applied to the list as it is now. `i` is the
 *  row's place when the edit started: a second edit in the same tick, before the list
 *  redraws, still finds its row (a lookup by the old row object would not). */
export function editRow<T extends Record<string, unknown>>(items: T[], i: number, seen: T, next: T): T[] {
  return items.map((x, j) => (j === i ? applyEdit(x, seen, next) : x));
}
