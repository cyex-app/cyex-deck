// K3 A boosted player's sound runs through Web Audio (gain.ts), and Chromium's A/V sync
// doesn't count that path's latency: the sound comes out baseLatency + outputLatency, plus
// the limiter's lookahead, after the picture (50 ms measured). It stays routed at 100 % too.
// So the picture is held back by as much: hls.js's video appends get a later timestampOffset
// and the browser shows every frame that much later. Nothing runs per frame.

/** The limiter's lookahead (DynamicsCompressorNode, 6 ms in Chromium). */
export const LOOKAHEAD = 0.006;
/** More than this is a broken reading, not a device. */
export const MAX_DELAY = 0.3;
/** A new reading moves the delay only when it's this far off, so jitter never flushes. */
export const STEP = 0.01;

/** Seconds the boosted sound trails the picture, rounded to the millisecond. */
export function measured(base: number, output: number): number {
  const sum = (Number.isFinite(base) ? base : 0) + (Number.isFinite(output) ? output : 0);
  if (sum <= 0) return 0;
  return Math.round(Math.min(sum + LOOKAHEAD, MAX_DELAY) * 1000) / 1000;
}

/** The delay to use next, from the one in use and a fresh reading. */
export function settle(applied: number, now: number): number {
  return Math.abs(now - applied) >= STEP ? now : applied;
}

/** The timestampOffset hls.js should give an append: video moves later, the rest stays. */
export function shifted(offset: number | undefined, type: string, delay: number): number | undefined {
  if (type !== 'video' || offset === undefined || !Number.isFinite(offset) || !delay) return offset;
  return offset + delay;
}

interface Appending {
  type: string;
  offset?: number;
  frag?: { sn?: number | string };
}

interface Triggers {
  trigger(event: string, data: unknown): boolean;
}

/** Holds back `hls`'s picture by `delay()` seconds (0 when the sound isn't in Web Audio,
 *  null while it can't be read yet: the one in use stays).
 *  Wraps `trigger` for BUFFER_APPENDING (`appending`), which hls.js reads the offset from.
 *  A delay that changes after video was appended calls `resync`: it reloads the buffer at
 *  the play head, so the frames buffered with the old one go at once (VODs hold 30 s).
 *  `resync` returns false when it can't (loading stopped): the next append tries again.
 *  Returns `check`, which reads the delay now (the boost graph changed), false while it's null. */
export function delayVideo(hls: Triggers, appending: string, delay: () => number | null, resync: () => boolean): () => boolean {
  const trigger = hls.trigger.bind(hls);
  let applied = 0;
  let appended = false;
  hls.trigger = (event, data) => {
    if (event === appending) {
      const d = data as Appending;
      if (d.type === 'video' && d.frag?.sn !== 'initSegment') {
        const now = delay();
        const next = now === null ? applied : settle(applied, now);
        // Outside hls.js's own dispatch: the reload flushes and restarts loading.
        if (next !== applied && appended) setTimeout(resync, 0);
        applied = next;
        appended = true;
        d.offset = shifted(d.offset, d.type, applied);
      }
    }
    return trigger(event, data);
  };
  return () => {
    const now = delay();
    if (now === null) return false;
    const next = settle(applied, now);
    if (next !== applied && (!appended || resync())) applied = next;
    return true;
  };
}
