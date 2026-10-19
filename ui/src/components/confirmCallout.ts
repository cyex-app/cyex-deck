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
