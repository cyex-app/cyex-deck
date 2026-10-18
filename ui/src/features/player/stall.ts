// d-4 A live stream that stops moving while it should play: after STALL_MS it says so and
// reopens, again STALL_MS after each try, until the picture moves.

export const STALL_MS = 10_000;
export const STALL_TEXT = 'Stream stalled. Trying again…';
/** rd-3 Once stalled, it takes this many checks (seconds) of steady play to clear: a reopen's
 *  new source plays a moment and stalls again, and the note flickered with each try. */
export const CLEAR_CHECKS = 3;

export interface StallState {
  /** Play head at the last check. */
  readonly pos: number;
  /** When it last moved (or stopped being watched). */
  readonly at: number;
  /** The last reopen. */
  readonly tried: number;
  readonly stalled: boolean;
  /** rd-3 Checks in a row with steady play since the stall. */
  readonly moving: number;
}

/** none: nothing to do. retry: show the note (if not yet) and reopen. clear: it moves again. */
export type StallAction = 'none' | 'retry' | 'clear';

export const stallStart = (now: number, pos = 0): StallState => ({ pos, at: now, tried: 0, stalled: false, moving: 0 });

/** One check, once a second. `active`: it should be playing (not paused by the user, held,
 *  failed or in an ad break). `ready`: the element plays with data (not paused, not seeking,
 *  enough buffered): only then a moved play head is progress. A reopen's new source or seek
 *  moves it without playing, and a jump of more than a few seconds is no play either. */
export function stallStep(
  s: StallState,
  o: { now: number; pos: number; active: boolean; ready: boolean },
): { s: StallState; action: StallAction } {
  if (!o.active) return { s: stallStart(o.now, o.pos), action: s.stalled ? 'clear' : 'none' };
  const d = o.pos - s.pos;
  const moved = o.ready && d > 0.05 && (!s.stalled || d < 3);
  if (!s.stalled) {
    if (moved) return { s: stallStart(o.now, o.pos), action: 'none' };
    const next = { ...s, pos: o.pos };
    if (o.now - s.at < STALL_MS) return { s: next, action: 'none' };
    return { s: { ...next, stalled: true, tried: o.now, moving: 0 }, action: 'retry' };
  }
  const moving = moved ? s.moving + 1 : 0;
  if (moving >= CLEAR_CHECKS) return { s: stallStart(o.now, o.pos), action: 'clear' };
  const next = { ...s, pos: o.pos, moving };
  // rd-3 STALL_MS after the last try; not while it plays on.
  if (o.now - s.tried < STALL_MS || moving > 0) return { s: next, action: 'none' };
  return { s: { ...next, tried: o.now, moving: 0 }, action: 'retry' };
}
