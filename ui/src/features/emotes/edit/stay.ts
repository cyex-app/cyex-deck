// c-2 V106-31/32 When the emote card (or a popover with a name form) ignores a wheel turn or a
// lost window.

/** Marks a name field that holds text; a box with one stays open. */
export const DRAFT = '[data-draft]';

interface Box {
  contains(node: never): boolean;
  querySelector(selectors: string): unknown;
}

/** True when `e` shouldn't close `box`: a typed name inside, or a wheel turn over the box itself. */
export function stayOpen(box: Box | undefined, e: { type: string; target: unknown }): boolean {
  if (!box) return false;
  if (box.querySelector(DRAFT)) return true;
  return e.type === 'wheel' && box.contains(e.target as never);
}
