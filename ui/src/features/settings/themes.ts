// U8 Names and checks for the theme editor.

/** Theme colors (`cyex_core::palette::KEYS`) with the names the editor shows. */
export const COLOR_NAMES: readonly (readonly [string, string])[] = [
  ['bg', 'Background'],
  ['surface', 'Panels'],
  ['elevated', 'Menus and cards'],
  ['border', 'Lines'],
  ['text', 'Text'],
  ['muted', 'Secondary text'],
  ['dim', 'Faint text'],
  ['red', 'Red'],
  ['yellow', 'Yellow'],
  ['green', 'Green'],
  ['cyan', 'Cyan'],
];

const NAMES = new Map(COLOR_NAMES);

/** "Text and Faint text were nudged …" for the colors Rust had to change. */
export function adjustedNote(keys: readonly string[]): string {
  const names = keys.map((k) => NAMES.get(k) ?? k);
  if (!names.length) return '';
  const list = names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
  return `${list} ${names.length === 1 ? 'was' : 'were'} nudged so everything stays readable.`;
}

/** An https link Rust will take as a wallpaper (no quotes, brackets or spaces). */
export function isLink(source: string): boolean {
  return /^https:\/\/[^\s"'()\\]+$/.test(source) && source.length <= 2048;
}

/** T2 `#rrggbb` from what was typed (`#abc`, `abc` and `#AABBCC` all work); null when it isn't a color. */
export function parseHex(text: string): string | null {
  const m = /^#?([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(text.trim());
  if (!m) return null;
  const h = m[1].toLowerCase();
  return `#${h.length === 3 ? [...h].map((c) => c + c).join('') : h}`;
}

/** T1 The palette's own color for `key` when it is a plain hex, else null. */
export function appliedHex(value: string | undefined): string | null {
  return value && /^#[0-9a-f]{6}$/i.test(value) ? value.toLowerCase() : null;
}

/** T1 "Kept dark so text stays readable": why a picked color isn't the one painted. */
export function keptNote(key: string, base: 'dark' | 'light'): string {
  return ['bg', 'surface', 'elevated', 'border'].includes(key)
    ? `Kept ${base} so text stays readable`
    : 'Nudged so it stays readable';
}
