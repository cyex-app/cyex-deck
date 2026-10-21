// V107-R33 The mouse's back and forward buttons walk the places shown before (PlaceHistory),
// wired once for a window. The window says where it is, which places still exist and how to
// go to one; this keeps the history and listens to the buttons.

import { PlaceHistory } from './placeHistory.ts';

export interface NavOptions<P> {
  same: (a: P, b: P) => boolean;
  /** Where the window is now, null when nowhere yet. */
  here: () => P | null;
  /** Whether a place can still be shown (a closed or popped-out tab can't). */
  ok: (p: P) => boolean;
  go: (p: P) => void;
}

/** What `listen` needs of `window`. */
export interface NavTarget {
  addEventListener(type: 'mousedown' | 'mouseup', f: (e: MouseEvent) => void): void;
  removeEventListener(type: 'mousedown' | 'mouseup', f: (e: MouseEvent) => void): void;
}

/** Buttons 3 and 4: back and forward. */
const isNav = (e: MouseEvent) => e.button === 3 || e.button === 4;

export function navHistory<P>(o: NavOptions<P>) {
  const history = new PlaceHistory<P>(o.same);
  let walking = false;
  const walk = (forward: boolean): void => {
    const now = o.here();
    const to = forward ? history.goForward(now, o.ok) : history.goBack(now, o.ok);
    if (!to) return;
    walking = true;
    try {
      o.go(to);
    } finally {
      walking = false;
    }
  };
  return {
    /** The window is leaving `from` for somewhere new (not while walking the history). */
    left(from: P): void {
      if (!walking) history.left(from);
    },
    walk,
    /** Listens to the buttons (WebView2 would otherwise navigate). Returns the stop. */
    listen(target: NavTarget): () => void {
      const down = (e: MouseEvent) => {
        if (isNav(e)) e.preventDefault();
      };
      const up = (e: MouseEvent) => {
        if (!isNav(e)) return;
        e.preventDefault();
        walk(e.button === 4);
      };
      target.addEventListener('mousedown', down);
      target.addEventListener('mouseup', up);
      return () => {
        target.removeEventListener('mousedown', down);
        target.removeEventListener('mouseup', up);
      };
    },
  };
}

/** For a component: wires the buttons on `window` while it lives. */
export function useNavHistory<P>(o: NavOptions<P>) {
  const nav = navHistory(o);
  $effect(() => nav.listen(window));
  return nav;
}
