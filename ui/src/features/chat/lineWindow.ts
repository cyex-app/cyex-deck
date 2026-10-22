// V107-M6 Only part of the chat is drawn: every line stays in the room's data, the list
// draws the newest WINDOW of them. Scrolling up near the top draws STEP more; back at the
// bottom it shrinks to WINDOW again. While scrolled up, the window holds its first line
// (`from`), so new lines below and history put in above never move what's drawn.

export const WINDOW = 250;
export const STEP = 150;
/** Lines drawn above the one a jump goes to, so it doesn't sit at the very top. */
export const JUMP_MARGIN = 50;
/** Following this long shrinks the window back. */
export const SHRINK_MS = 1500;

type Item = { id: string; ts: number };

/** The window's first line, by id; `ts` finds where it was once that line is gone. */
export type From = { id: string; ts: number } | null;

/** Index of the first drawn line. `null`: the newest `size`. */
export function startOf(list: readonly Item[], from: From, size = WINDOW): number {
  if (!from) return Math.max(0, list.length - size);
  const at = list.findIndex((m) => m.id === from.id);
  if (at !== -1) return at;
  // Gone (dropped off the top, or taken back): the first line that came after it.
  const next = list.findIndex((m) => m.ts >= from.ts);
  return next === -1 ? Math.max(0, list.length - size) : next;
}

const fromAt = (list: readonly Item[], i: number): From => (list[i] ? { id: list[i].id, ts: list[i].ts } : null);

/** Holds the window where it is (scrolled up). */
export function hold(list: readonly Item[], from: From, size = WINDOW): From {
  return from ?? fromAt(list, startOf(list, null, size));
}

/** `step` more older lines, or `undefined` when everything is drawn already. */
export function older(list: readonly Item[], from: From, step = STEP, size = WINDOW): From | undefined {
  const start = startOf(list, from, size);
  if (start <= 0) return undefined;
  return fromAt(list, Math.max(0, start - step));
}

/** A window that includes line `index` (a jump), or `undefined` when it's drawn already. */
export function around(list: readonly Item[], from: From, index: number, margin = JUMP_MARGIN, size = WINDOW): From | undefined {
  if (index < 0 || index >= startOf(list, from, size)) return undefined;
  return fromAt(list, Math.max(0, index - margin));
}
