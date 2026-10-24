// V107-Mk What the chat identity popover offers: Twitch's named colors (the same list as
// `NAMED` in app/src/chat/identity.rs, checked there) and a free color with Prime or Turbo.
// Smiley styles come from Rust, already empty when there's nothing to pick.

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

/** A grid cell's middle on screen. */
export interface Cell {
  x: number;
  y: number;
}

/**
 * V108-ID Where a key moves focus in a wrapped grid of picks (badges, colors, smileys): left and
 * right step, Home and End jump, up and down go to the nearest cell in the row above or below,
 * measured, so a wide cell (None) doesn't throw the columns off. Stays put at an edge; null
 * when the key isn't ours. Moving never picks: Enter or Space does.
 */
export function gridMove(key: string, at: number, cells: readonly Cell[]): number | null {
  const n = cells.length;
  if (!n || at < 0 || at >= n) return null;
  switch (key) {
    case 'ArrowLeft':
      return Math.max(at - 1, 0);
    case 'ArrowRight':
      return Math.min(at + 1, n - 1);
    case 'Home':
      return 0;
    case 'End':
      return n - 1;
    case 'ArrowUp':
    case 'ArrowDown': {
      const { x, y } = cells[at];
      const up = key === 'ArrowUp';
      const rows = cells.map((c) => c.y).filter((cy) => (up ? cy < y - 1 : cy > y + 1));
      if (!rows.length) return at;
      const row = up ? Math.max(...rows) : Math.min(...rows);
      let best = at;
      let dist = Infinity;
      cells.forEach((c, i) => {
        const d = Math.abs(c.x - x);
        if (Math.abs(c.y - row) <= 1 && d < dist) [best, dist] = [i, d];
      });
      return best;
    }
    default:
      return null;
  }
}
