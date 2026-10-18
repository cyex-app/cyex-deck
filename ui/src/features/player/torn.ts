// h3 A live stream's clock that jumps without a discontinuity mark: a prefetched segment
// (low latency) listed before Twitch showed it starts a new timeline (an ad, a restart, a
// reconnect). hls.js glues the new times onto the old ones in the same discontinuity: the
// sound lands minutes back, the picture freezes where the sound runs out, and the shifted
// fragment times pull the play head back. Caught when the fragment is parsed, a new hls.js
// instance on the session takes over where the play head reaches it.

/** Seconds a parsed fragment may sit off its playlist spot (real drift is milliseconds). */
export const TORN_S = 2;
/** How close to the torn fragment the play head gets before the new instance takes over. */
export const TORN_LEAD_S = 0.25;

/** The parsed track `start` is far from where the playlist put the fragment (`planned`). */
export function isTorn(planned: number | undefined, start: number): boolean {
  return planned !== undefined && Number.isFinite(start) && Math.abs(start - planned) > TORN_S;
}

/** Time for the play head to leave: it reached the torn fragment's spot. */
export function tornDue(at: number | null, current: number): boolean {
  return at !== null && current >= at - TORN_LEAD_S;
}

/** Where the new instance starts: the torn fragment when the live point falls before it, so
 *  no fragment of the old timeline plays again (and tears again); null keeps the live point. */
export function tornStart(frags: readonly { sn: number | 'initSegment'; start: number }[], sn: number | null, sync: number | null): number | null {
  if (sn === null || sync === null) return null;
  const f = frags.find((x) => x.sn === sn);
  return f && f.start > sync ? f.start : null;
}
