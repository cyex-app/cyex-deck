// V108-N24 The replay's clock: hls.js places its segments in playlist seconds, the rewind
// works in seconds since the stream started. Each segment carries its program date time.

/** What hls.js knows of a segment (its `Fragment`). */
export interface Piece {
  readonly sn: number | 'initSegment';
  readonly start: number;
  readonly duration: number;
  readonly programDateTime: number | null;
}

const dated = (f: Piece): f is Piece & { programDateTime: number } => typeof f.sn === 'number' && f.programDateTime !== null;

/** The date (ms) at playlist position `pos`, null when no segment there has one. */
export function dateAt(pieces: readonly Piece[], pos: number): number | null {
  let best: (Piece & { programDateTime: number }) | null = null;
  for (const f of pieces) {
    if (!dated(f)) continue;
    if (f.start <= pos) best = f;
    else break;
  }
  if (!best) return null;
  return best.programDateTime + Math.min(Math.max(0, pos - best.start), best.duration) * 1000;
}

/** The playlist position of `date` (ms). In a hole: the next segment's start; before the
 *  first or past the last: that end. null without any dated segment. */
export function posAt(pieces: readonly Piece[], date: number): number | null {
  const list = pieces.filter(dated);
  if (!list.length) return null;
  for (const f of list) {
    const end = f.programDateTime + f.duration * 1000;
    if (date < f.programDateTime) return f.start;
    if (date < end) return f.start + (date - f.programDateTime) / 1000;
  }
  return list[list.length - 1]!.start;
}

/** V108-N24b Where the replay's one start loads from: `date` (ms) placed in the playlist hls.js
 *  just read (the ring may have trimmed its front since `offset` was read), else `offset`. */
export function replayStart(pieces: readonly Piece[] | undefined, date: number, offset: number): number {
  return posAt(pieces ?? [], date) ?? offset;
}

/** Seconds since `origin` (ms) of the segment after `sn`, null when it's the last. */
export function nextAfter(pieces: readonly Piece[], sn: number, origin: number): number | null {
  const next = pieces.find((f) => dated(f) && (f.sn as number) > sn);
  return next?.programDateTime != null ? (next.programDateTime - origin) / 1000 : null;
}

/** V108-N24 Whether the play head has come to a missing segment that starts at `start`
 *  (playlist seconds): there, or out of buffer before it. Ahead of it, the buffer plays out
 *  first. `end`: the end of the buffered range the play head is in, null when it's in none. */
export function goneDue(pos: number, end: number | null, start: number): boolean {
  return pos >= start - 0.5 || end === null || end - pos < 0.5;
}

/** V108-N24 A load error that means Twitch doesn't have the segment (any other 4xx doesn't). */
export const isGone = (code: number): boolean => code === 404 || code === 410;
