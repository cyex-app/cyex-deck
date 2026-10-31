// V108-LL2 Closer to live by speed, as twitch.tv does: with low latency at live, the player plays a
// little faster while the buffer holds spare seconds and a little slower when it runs thin. The
// buffer swings between loads (a whole segment, or an fMP4 segment's held second, V108-LL3); the
// least of it over a few seconds is what is spare.

/** s of samples the spare buffer is judged over: two segments' worth of swing. */
export const STEER_WINDOW = 4;
/** Least buffer over the window above which it speeds up, and the one it settles back to 1× at. */
export const SPARE_FAST = 1.2;
export const SPARE_DONE = 0.8;
/** Least buffer under which it slows down, and the one it settles back to 1× at. */
export const SPARE_LOW = 0.4;
export const SPARE_SAFE = 0.7;
export const RATE_FAST = 1.05;
export const RATE_SLOW = 0.95;

export class Steer {
  private samples: { at: number; ahead: number }[] = [];
  rate = 1;

  /** The rate for a sample of `ahead` s buffered at `now` (s). */
  step(now: number, ahead: number): number {
    this.samples.push({ at: now, ahead });
    while (this.samples.length && this.samples[0].at < now - STEER_WINDOW) this.samples.shift();
    // A full window first: a fresh start or seek has no swing to read yet.
    if (now - this.samples[0].at < STEER_WINDOW - 0.5) return this.rate;
    const spare = Math.min(...this.samples.map((s) => s.ahead));
    if (this.rate > 1) this.rate = spare <= SPARE_DONE ? 1 : this.rate;
    else if (this.rate < 1) this.rate = spare >= SPARE_SAFE ? 1 : this.rate;
    if (this.rate === 1) this.rate = spare > SPARE_FAST ? RATE_FAST : spare < SPARE_LOW ? RATE_SLOW : 1;
    return this.rate;
  }

  /** Off (paused, rewound, a jump): back to 1×, the window starts again. */
  reset(): void {
    this.samples = [];
    this.rate = 1;
  }
}
