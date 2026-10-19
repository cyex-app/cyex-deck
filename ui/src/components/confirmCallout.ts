// V107-P4 Where a confirm callout sits and how long it waits: under its button, the notch
// pointing at the button's middle, inside the window.

import type { Rect } from '../lib/place';

/** Height of the notch between the button and the box. */
export const NOTCH = 5;
/** The callout goes away by itself after this long. */
export const CONFIRM_MS = 3000;

/** The box's left and top, and the notch's x inside it. Centered under the button when it
 * fits, else pushed inside the window; the notch stays on the button. */
export function calloutAt(anchor: Rect, width: number, viewWidth: number, edge = 6): { left: number; top: number; notch: number } {
  const mid = (anchor.left + anchor.right) / 2;
  const left = Math.max(edge, Math.min(mid - width / 2, viewWidth - edge - width));
  const notch = Math.max(NOTCH + 3, Math.min(mid - left, width - NOTCH - 3));
  return { left: Math.round(left), top: Math.round(anchor.bottom + NOTCH + 1), notch: Math.round(notch) };
}

/** V107-P5 Over a menu item instead: the box ends 1 px into the item's top edge, so the
 * pointer can go from the item to the box without crossing the item above, and the notch
 * points down into the item. Under it when there's no room above. */
export function calloutAbove(
  anchor: Rect,
  width: number,
  height: number,
  viewWidth: number,
  edge = 6,
): { left: number; top: number; notch: number; below: boolean } {
  const x = calloutAt(anchor, width, viewWidth, edge);
  const top = anchor.top + 1 - height;
  if (top < edge) return { ...x, top: Math.round(anchor.bottom - 1), below: true };
  return { ...x, top: Math.round(top), below: false };
}

export interface Timers {
  set: (run: () => void, ms: number) => unknown;
  clear: (id: unknown) => void;
}

const REAL: Timers = {
  set: (run, ms) => setTimeout(run, ms),
  clear: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};

/** Calls `end` once after `ms`, unless canceled first. */
export function expiry(end: () => void, ms = CONFIRM_MS, t: Timers = REAL): { cancel: () => void } {
  let live = true;
  const id = t.set(() => {
    if (live) {
      live = false;
      end();
    }
  }, ms);
  return {
    cancel() {
      if (!live) return;
      live = false;
      t.clear(id);
    },
  };
}
