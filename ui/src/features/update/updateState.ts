// B4 The badge's start state comes from Rust, so "Later" holds after a reload and in other windows.

import type { Update } from '../../ipc/bindings/Update';
import type { UpdateState } from '../../ipc/bindings/UpdateState';

/** `pending` is an `update://available` that came in while the state was read: it's newer. */
export function startState(s: UpdateState | null, pending: Update | null | undefined): UpdateState {
  return {
    update: pending !== undefined ? pending : (s?.update ?? null),
    later: s?.later ?? false,
  };
}

/** V1061 The pop-over opens by itself once per version, never after "Later". */
export function opensItself(version: string | undefined, shown: string | null, later: boolean): boolean {
  return !!version && version !== shown && !later;
}
