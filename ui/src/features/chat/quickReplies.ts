// V108-N20 How a saved phrase reads in the quick replies menu.

/** Longest line the menu shows, in characters the reader sees. */
export const MENU_MAX = 60;

const graphemes = (s: string): string[] =>
  typeof Intl.Segmenter === 'function' ? Array.from(new Intl.Segmenter(undefined, { granularity: 'grapheme' }).segment(s), (p) => p.segment) : Array.from(s);

/** The phrase for the menu: cut after 59 characters with …, never inside an emoji or
 *  surrogate pair. `cut` says whether it was shortened. */
export function menuLine(text: string): { label: string; cut: boolean } {
  if (text.length <= MENU_MAX) return { label: text, cut: false };
  const g = graphemes(text);
  if (g.length <= MENU_MAX) return { label: text, cut: false };
  return { label: `${g.slice(0, MENU_MAX - 1).join('')}…`, cut: true };
}
