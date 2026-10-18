// d-4 A live stream that stops moving while it should play: after STALL_MS it says so and
// reopens, again STALL_MS after each try, until the picture moves.

export const STALL_MS = 10_000;
export const STALL_TEXT = 'Stream stalled. Trying again…';
/** rd-3 Once stalled, it takes this many checks (seconds) of steady play to clear: a reopen's
 *  new source plays a moment and stalls again, and the note flickered with each try. */
export const CLEAR_CHECKS = 3;

/** qd-3 Once stalled, new content clears it after this many checks: it plays past anything shown
 *  before, so it can't be a reopen replaying the last segments. */
export const CLEAR_NEW_CHECKS = 2;
/** qd-3 How far past the newest shown content (program date time) counts as new, in ms. */
export const NEW_MS = 1000;
/** qd-3 A playlist that ends this close to the stuck picture: the stream stopped, not the loading. */
export const EDGE_NEAR_MS = 3000;

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
  /** qd-3 Newest content shown (program date time, ms), null when the stream has none. */
  readonly seen: number | null;
  /** qd-3 `seen` when the stall began: only content past it is progress. */
  readonly frontier: number | null;
  /** qd-3 The playlist's newest content at the last try. */
  readonly edgeAt: number | null;
}

/** none: nothing to do. retry: show the note (if not yet) and reopen. clear: it moves again. */
export type StallAction = 'none' | 'retry' | 'clear';

export const stallStart = (now: number, pos = 0, seen: number | null = null): StallState => ({
  pos,
  at: now,
  tried: 0,
  stalled: false,
  moving: 0,
  seen,
  frontier: null,
  edgeAt: null,
});

const later = (a: number | null, b: number | null | undefined): number | null => (b == null ? a : a === null ? b : Math.max(a, b));

/** One check, once a second. `active`: it should be playing (not paused by the user, held,
 *  failed or in an ad break). `ready`: the element plays with data (not paused, not seeking,
 *  enough buffered): only then a moved play head is progress. A reopen's new source or seek
 *  moves it without playing, and a jump of more than a few seconds is no play either.
 *  qd-3 `date`: the program date time at the play head; `edge`: the playlist's newest. Once
 *  stalled, only content newer than any shown before is progress (a reopen of a stuck stream
 *  replays its last segments), and a playlist that grew since the last try retries at once. */
export function stallStep(
  s: StallState,
  o: { now: number; pos: number; active: boolean; ready: boolean; date?: number | null; edge?: number | null },
): { s: StallState; action: StallAction } {
  const date = o.ready ? (o.date ?? null) : null;
  const seen = later(s.seen, date);
  if (!o.active) return { s: stallStart(o.now, o.pos, seen), action: s.stalled ? 'clear' : 'none' };
  const d = o.pos - s.pos;
  if (!s.stalled) {
    const moved = o.ready && d > 0.05;
    if (moved) return { s: stallStart(o.now, o.pos, seen), action: 'none' };
    const next = { ...s, pos: o.pos, seen };
    if (o.now - s.at < STALL_MS) return { s: next, action: 'none' };
    return { s: { ...next, stalled: true, tried: o.now, moving: 0, frontier: s.seen, edgeAt: o.edge ?? null }, action: 'retry' };
  }
  const byDate = s.frontier !== null && date !== null;
  // qd-3 New content counts from its first check, even after a jump: hls.js skips to the
  // edge when the playlist goes on, and a reopen's new source lands there too.
  const moved = byDate ? date > s.frontier! + NEW_MS && d > 0.05 : o.ready && d > 0.05 && d < 3;
  const moving = moved ? s.moving + 1 : 0;
  if (moving >= (byDate ? CLEAR_NEW_CHECKS : CLEAR_CHECKS)) return { s: stallStart(o.now, o.pos, seen), action: 'clear' };
  const next = { ...s, pos: o.pos, moving, seen };
  if (moving > 0) return { s: next, action: 'none' };
  // qd-3 The stream itself had stopped (its playlist ended at the stuck picture) and goes on
  // again: try now, not at the next 10 s. A playlist that grows while nothing loads waits.
  const stopped = s.edgeAt !== null && s.frontier !== null && s.edgeAt <= s.frontier + EDGE_NEAR_MS;
  const grew = stopped && o.edge != null && o.edge > s.edgeAt! + NEW_MS && o.now - s.tried >= 2000;
  // rd-3 STALL_MS after the last try; not while it plays on.
  if (o.now - s.tried < STALL_MS && !grew) return { s: next, action: 'none' };
  return { s: { ...next, tried: o.now, moving: 0, edgeAt: o.edge ?? s.edgeAt }, action: 'retry' };
}
