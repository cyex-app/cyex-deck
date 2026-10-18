// d-4 A live stream that stops moving while it should play: after STALL_MS it says so and
// reopens, again every STALL_MS, until the picture moves.

export const STALL_MS = 10_000;
export const STALL_TEXT = 'Stream stalled. Trying again…';

export interface StallState {
  /** Play head at the last check. */
  readonly pos: number;
  /** When it last moved (or stopped being watched). */
  readonly at: number;
  /** The last reopen. */
  readonly tried: number;
  readonly stalled: boolean;
}

/** none: nothing to do. retry: show the note (if not yet) and reopen. clear: it moves again. */
export type StallAction = 'none' | 'retry' | 'clear';

export const stallStart = (now: number, pos = 0): StallState => ({ pos, at: now, tried: 0, stalled: false });

/** One check. `active`: it should be playing (not paused by the user, held, failed or in an ad
 *  break). `ready`: the element plays with data (not paused, not seeking, enough buffered): only
 *  then a moved play head is progress. A reopen's new source or seek moves it without playing. */
export function stallStep(
  s: StallState,
  o: { now: number; pos: number; active: boolean; ready: boolean },
): { s: StallState; action: StallAction } {
  if (!o.active || (o.ready && o.pos - s.pos > 0.05)) return { s: stallStart(o.now, o.pos), action: s.stalled ? 'clear' : 'none' };
  const next = { ...s, pos: o.pos };
  if (!s.stalled) {
    if (o.now - s.at < STALL_MS) return { s: next, action: 'none' };
    return { s: { ...next, stalled: true, tried: o.now }, action: 'retry' };
  }
  if (o.now - s.tried < STALL_MS) return { s: next, action: 'none' };
  return { s: { ...next, tried: o.now }, action: 'retry' };
}
