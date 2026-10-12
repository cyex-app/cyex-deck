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
