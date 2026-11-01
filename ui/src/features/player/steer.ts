// V108-LL2 Closer to live by speed, as twitch.tv does: with low latency at live, the player plays a
// little faster while the buffer holds spare seconds and a little slower when it runs thin. The
// buffer swings between loads (a whole segment, or an fMP4 segment's held second, V108-LL3); the
// least of it over a few seconds is what is spare.
// V108-LL6 A stream that once ran dry (a segment came a second or more late, 2026/11/01 on xQc:
// 9 in 20 min) keeps more spare from then on: each dry run adds MARGIN_STEP to every threshold,
// which wears off over minutes of clean play. Streams that arrive on time keep the small spare.

/** s of samples the spare buffer is judged over: two segments' worth of swing. */
export const STEER_WINDOW = 4;
/** Least buffer over the window above which it speeds up, and the one it settles back to 1× at.
 *  V108-LL4 0.9/0.5 (was 1.2/0.8): about 0.35 s closer, 0 stalls in 180 s runs; 0.7/0.35 stalled. */
export const SPARE_FAST = 0.9;
export const SPARE_DONE = 0.5;
/** Least buffer under which it slows down, and the one it settles back to 1× at (was 0.4/0.7). */
export const SPARE_LOW = 0.25;
export const SPARE_SAFE = 0.4;
export const RATE_FAST = 1.05;
export const RATE_SLOW = 0.95;
/** V108-LL6 s of spare added per dry run, the most it adds, and how fast it wears off (s per s:
 *  one step in 10 min). Halved (was 0.5 / 1.5) by the user: less delay on a bad stream. */
export const MARGIN_STEP = 0.25;
export const MARGIN_MAX = 0.75;
export const MARGIN_DECAY = MARGIN_STEP / 600;
/** V108-LL6 s at 1× before it speeds up again: the speed changed every few seconds at the band's
 *  edge (fast, 1×, slow within a second before a stall). */
export const REST = 3;

export class Steer {
  private samples: { at: number; ahead: number }[] = [];
  rate = 1;
  /** V108-LL6 Spare s added to every threshold after dry runs. */
  margin = 0;
  private marginAt: number | null = null;
  /** When it last went back to 1×. */
  private restAt = -Infinity;

  /** The rate for a sample of `ahead` s buffered at `now` (s). */
  step(now: number, ahead: number): number {
    this.wear(now);
    this.samples.push({ at: now, ahead });
    while (this.samples.length && this.samples[0].at < now - STEER_WINDOW) this.samples.shift();
    // A full window first: a fresh start or seek has no swing to read yet.
    if (now - this.samples[0].at < STEER_WINDOW - 0.5) return this.rate;
    const spare = Math.min(...this.samples.map((s) => s.ahead));
    const m = this.margin;
    const was = this.rate;
    if (this.rate > 1) this.rate = spare <= SPARE_DONE + m ? 1 : this.rate;
    else if (this.rate < 1) this.rate = spare >= SPARE_SAFE + m ? 1 : this.rate;
    if (this.rate === 1) {
      if (spare < SPARE_LOW + m) this.rate = RATE_SLOW;
      else if (spare > SPARE_FAST + m && now - this.restAt >= REST) this.rate = RATE_FAST;
    }
    if (this.rate === 1 && was !== 1) this.restAt = now;
    return this.rate;
  }

  /** V108-LL6 It is steering (sampled since the last reset). */
  get steering(): boolean {
    return this.samples.length > 0;
  }

  /** V108-LL6 The buffer ran dry while it steered: more spare from now on. */
  dry(now: number): void {
    this.wear(now);
    this.margin = Math.min(MARGIN_MAX, this.margin + MARGIN_STEP);
  }

  /** Off (paused, rewound, a jump): back to 1×, the window starts again. The margin stays: it
   *  belongs to the stream. */
  reset(): void {
    this.samples = [];
    this.rate = 1;
  }

  private wear(now: number): void {
    if (this.marginAt !== null) this.margin = Math.max(0, this.margin - Math.max(0, now - this.marginAt) * MARGIN_DECAY);
    this.marginAt = now;
  }
}
