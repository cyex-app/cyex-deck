// V107-P5 A menu item with `confirm` asks once more before it runs: the first pick arms it
// (the menu stays open and a callout over the item says e.g. "Click to delete"), a second
// pick runs it. V108-C21 A second pointer click on the same item does not run it (only the
// bubble does, or the keyboard); the timer pauses while the pointer is on the bubble. Disarms after 3 s, on Escape, or when another item is hovered, focused or
// picked. Pure state, so the timer can be faked in tests; `menu.svelte.ts` draws from it.

import { CONFIRM_MS, expiry, type Expiry, type Timers } from './confirmCallout.ts';

export interface Confirmer<T> {
  readonly armed: T | null;
  /** True: run it now. False: it's armed and waits for a second pick. */
  press(item: T, confirm: string | undefined, viaKey?: boolean): boolean;
  /** V108-C21 The pointer is on / left the bubble. */
  hold(on: boolean): void;
  /** The pointer or focus went to `item`: anything else that's armed lets go. */
  moveTo(item: T): void;
  /** Lets go; true when something was armed (Escape then stops there). */
  cancel(): boolean;
}

export function confirmer<T>(onchange: (armed: T | null) => void, t?: Timers): Confirmer<T> {
  let armed: T | null = null;
  let timer: Expiry | null = null;
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
    press(item, confirm, viaKey = false) {
      if (!confirm) {
        if (armed !== null) set(null);
        return true;
      }
      if (armed === item) {
        if (!viaKey) return false;
        set(null);
        return true;
      }
      set(item);
      return false;
    },
    hold(on) {
      if (on) timer?.pause();
      else timer?.resume();
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
