// V107-P5 A menu item with `confirm` asks once more before it runs: the first pick arms it
// (the menu stays open and a callout over the item says e.g. "Click to delete"), a second
// pick runs it. Disarms after 3 s, on Escape, or when another item is hovered, focused or
// picked. Pure state, so the timer can be faked in tests; `menu.svelte.ts` draws from it.

import { CONFIRM_MS, expiry, type Timers } from './confirmCallout.ts';

export interface Confirmer<T> {
  readonly armed: T | null;
  /** True: run it now. False: it's armed and waits for a second pick. */
  press(item: T, confirm: string | undefined): boolean;
  /** The pointer or focus went to `item`: anything else that's armed lets go. */
  moveTo(item: T): void;
  /** Lets go; true when something was armed (Escape then stops there). */
  cancel(): boolean;
}

export function confirmer<T>(onchange: (armed: T | null) => void, t?: Timers): Confirmer<T> {
  let armed: T | null = null;
  let timer: { cancel: () => void } | null = null;
  function set(next: T | null) {
    timer?.cancel();
    timer = next === null ? null : expiry(() => set(null), CONFIRM_MS, t);
    if (armed === next) return;
    armed = next;
    onchange(next);
  }
  return {
    get armed() {
      return armed;
    },
    press(item, confirm) {
      if (!confirm || armed === item) {
        if (armed !== null) set(null);
        return true;
      }
      set(item);
      return false;
    },
    moveTo(item) {
      if (armed !== null && armed !== item) set(null);
    },
    cancel() {
      if (armed === null) return false;
      set(null);
      return true;
    },
  };
}
