// c-3 K14 Where focus goes when the card's name form closes and the card stays open.

interface Focusable {
  focus(): void;
}
interface Root {
  querySelector(selectors: string): Focusable | null;
}

/** The button that opened the form, when it's still there and enabled; else `home` (the card's
 *  heading). Returns what got focus. */
export function refocus(root: Root | undefined, opener: string | null, home: Focusable | undefined): Focusable | null {
  const back = opener ? (root?.querySelector(`[data-opener="${opener}"] button:not(:disabled)`) ?? null) : null;
  const to = back ?? home ?? null;
  to?.focus();
  return to;
}
