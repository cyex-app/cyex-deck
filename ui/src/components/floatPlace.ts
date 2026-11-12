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
