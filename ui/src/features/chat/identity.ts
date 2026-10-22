// V107-Mk What the chat identity popover offers: Twitch's named colors, a free color with
// Prime or Turbo, and the smiley styles Twitch lists for the account.

import type { SmileyChoice } from '../../ipc/bindings/SmileyChoice';
import type { Smilies } from '../../ipc/bindings/Smilies';
import type { SmileyStyle } from '../../ipc/bindings/SmileyStyle';

export const NAMED: readonly [string, string][] = [
  ['Blue', '#0000FF'],
  ['Blue violet', '#8A2BE2'],
  ['Cadet blue', '#5F9EA0'],
  ['Chocolate', '#D2691E'],
  ['Coral', '#FF7F50'],
  ['Dodger blue', '#1E90FF'],
  ['Firebrick', '#B22222'],
  ['Golden rod', '#DAA520'],
  ['Green', '#008000'],
  ['Hot pink', '#FF69B4'],
  ['Orange red', '#FF4500'],
  ['Red', '#FF0000'],
  ['Sea green', '#2E8B57'],
  ['Spring green', '#00FF7F'],
  ['Yellow green', '#9ACD32'],
];

export const STYLE_NAME: Record<SmileyStyle, string> = { robot: 'Robot', glitch: 'Glitch', monkey: 'Monkey' };

/** `#RRGGBB`, upper case, or null. A missing `#` is fine. */
export function hexColor(input: string): string | null {
  const s = input.trim().replace(/^#?/, '#');
  return /^#[0-9a-fA-F]{6}$/.test(s) ? s.toUpperCase() : null;
}

/** The named color `hex` is, if any. */
export function namedColor(hex: string | null): string | null {
  const h = hex?.toUpperCase();
  return NAMED.find(([, v]) => v === h)?.[0] ?? null;
}

/** Styles to pick from: only what Twitch offered, and only when there's a choice. */
export function smileyChoices(s: Smilies | null | undefined): SmileyChoice[] {
  return s && s.choices.length > 1 ? s.choices : [];
}
