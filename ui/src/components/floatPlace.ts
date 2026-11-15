// V110-N13 Where a floating panel sits: dragged anywhere in the window, kept inside it
// (also when the window shrinks), `MARGIN` off each edge where it fits.

export interface Pt {
  x: number;
  y: number;
}

export interface Size {
  w: number;
  h: number;
}

export const MARGIN = 8;

/** `p` moved just enough that a box of `size` stays inside `view`. A box larger than the
 *  view keeps its top-left corner in (its header stays reachable). */
export function clampInto(p: Pt, size: Size, view: Size, margin = MARGIN): Pt {
  const fit = (v: number, len: number, room: number) => Math.max(margin, Math.min(v, room - len - margin));
  return { x: Math.round(fit(p.x, size.w, view.w)), y: Math.round(fit(p.y, size.h, view.h)) };
}

/** Where the `nth` open panel starts: centred, a fifth down, each next one a step lower
 *  and to the right so they don't hide each other. */
export function startAt(size: Size, view: Size, nth = 0): Pt {
  const step = 28 * nth;
  return clampInto({ x: (view.w - size.w) / 2 + step, y: view.h / 5 + step }, size, view);
}

/** V111-33 `size` held between `min` and the room left from `at` to the view's far edge. */
export function clampSize(size: Size, min: Size, at: Pt, view: Size, margin = MARGIN): Size {
  const fit = (len: number, lo: number, from: number, room: number) => Math.round(Math.max(Math.min(lo, room - margin * 2), Math.min(len, room - from - margin)));
  return { w: fit(size.w, min.w, at.x, view.w), h: fit(size.h, min.h, at.y, view.h) };
}

/** V111-33 Where a panel sat and how big it was, by key, until the window closes. */
export interface Placed {
  at: Pt;
  size: Size;
}

const placed = new Map<string, Placed>();
export const recall = (key: string): Placed | undefined => placed.get(key);
export const remember = (key: string, p: Placed): void => void placed.set(key, p);
