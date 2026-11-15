// V108-LL2 Closer to live by speed, as twitch.tv does: with low latency at live, the player plays a
// little faster while the buffer holds spare seconds and a little slower when it runs thin. The
// buffer swings between loads (a whole segment, or an fMP4 segment's held second, V108-LL3); the
// least of it over a few seconds is what is spare.
// V108-LL6 A stream that once ran dry (a segment came a second or more late, 2026/11/01 on xQc:
// 9 in 20 min) keeps more spare from then on: each dry run adds MARGIN_STEP to every threshold,
// which wears off over minutes of clean play. Streams that arrive on time keep the small spare.
// V111-38 The one way a stream earns more spare, at any level: a dry run or a reading under
// NEAR_DRY adds a step, CALM s of steady play take one off.

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
/** V108-LL6, V111-38 s of spare added per dry run or near-dry reading, and the most it adds.
 *  V111-38 replaces V111-34's fixed 1 s at 1080p and up: a large level that arrives on time keeps
 *  the small spare, one that runs thin earns up to the same 1 s in steps. */
export const MARGIN_STEP = 0.25;
export const MARGIN_MAX = 1;
/** V111-38 A reading under this (s buffered) while steering counts as a dry run: the margin
 *  grows before the picture stops, not after. Under SPARE_LOW, so the usual swing never reaches it. */
export const NEAR_DRY = 0.1;
/** V111-38 s after a step in which another dry or near-dry one adds nothing: one trough (or a
 *  near-dry reading and the stall it ends in) counts once. */
export const BUMP_GAP = STEER_WINDOW;
/** V111-38 s of steady steered play without a dry or near-dry reading after which one step comes
 *  off (was 10 min per step, V108-LL6): a stream that had one bad minute plays close again soon.
 *  Only time steering counts (paused, rewound or out for a while: not calm, just not watched).
 *  And only if the step wasn't used: the least reading over those CALM s stayed MARGIN_STEP over
 *  NEAR_DRY (a step less would not have run near dry), else the count starts again. Mock, late
 *  segments every 30 s: without that rule it stepped down between them and stalled again. */
export const CALM = 60;
/** V108-LL6 s at 1× before it speeds up again: the speed changed every few seconds at the band's
 *  edge (fast, 1×, slow within a second before a stall). */
export const REST = 3;
/** V108-P10 s of spare added while the player is out of sight (its tab in the background, the
 *  window minimized or covered). There nobody sees the latency, but the page's timers are late
 *  (WebView2 wakes a hidden page about once a second, measured up to 3 s and more), the next
 *  segment's load waits for them, and the drop to the lowest quality (V103-03) loads a new
 *  playlist first: with the usual 0.5-1 s spare both ran dry. It slows to RATE_SLOW until the
 *  spare is there (about 50 s); back in sight it speeds up to the usual spare. */
export const AWAY_SPARE = 2.5;
/** V111-38 Least buffer a Live press's jump to the edge leaves above the extra: the middle of the
 *  1× band (SPARE_DONE..SPARE_FAST), so it neither speeds up nor slows down right after
 *  (V111-32 kept SPARE_FAST, 0,2 s more). */
export const SPARE_KEEP = (SPARE_DONE + SPARE_FAST) / 2;

export class Steer {
  private samples: { at: number; ahead: number }[] = [];
  rate = 1;
  /** V108-LL6, V111-38 Spare s added to every threshold after dry or near-dry readings. */
  margin = 0;
  /** V111-38 When the margin last grew (s), and s of steady steered play since then or the last step off. */
  private bumpAt = -Infinity;
  private calm = 0;
  private calmLow = Infinity;
  /** V108-P10 Out of sight: AWAY_SPARE more spare. */
  away = false;
  /** When it last went back to 1×. */
  private restAt = -Infinity;

  /** The rate for a sample of `ahead` s buffered at `now` (s). */
  step(now: number, ahead: number): number {
    const last = this.samples.at(-1);
    // Only steady play counts as calm: a gap (paused, hidden page timers) doesn't.
    if (last && now > last.at && now - last.at <= 1) this.wear(now - last.at, ahead);
    this.samples.push({ at: now, ahead });
    while (this.samples.length && this.samples[0].at < now - STEER_WINDOW) this.samples.shift();
    // A full window first: a fresh start or seek has no swing to read yet.
    if (now - this.samples[0].at < STEER_WINDOW - 0.5) return this.rate;
    // Out of sight the page's own timers run late (V108-P10): no word on the stream.
    if (ahead < NEAR_DRY && !this.away) this.dry(now);
    const spare = Math.min(...this.samples.map((s) => s.ahead));
    const m = this.extra;
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

  /** s of spare kept above the base bands: the margin this stream earned (V108-LL6, V111-38) and
   *  out of sight (V108-P10). The Live press keeps the same (V111-32). */
  get extra(): number {
    return this.margin + (this.away ? AWAY_SPARE : 0);
  }

  /** V111-32, V111-38 Spare buffer a Live press's jump to the edge leaves: the middle of the 1×
   *  band plus the extra, so it neither speeds up nor slows down right after and never undoes
   *  what steering keeps. */
  get keep(): number {
    return SPARE_KEEP + this.extra;
  }

  /** V108-LL6 It is steering (sampled since the last reset). */
  get steering(): boolean {
    return this.samples.length > 0;
  }

  /** V108-LL6 The buffer ran dry (or, V111-38, nearly) while it steered: more spare from now on,
   *  once per BUMP_GAP. */
  dry(now: number): void {
    this.calm = 0;
    this.calmLow = Infinity;
    if (now - this.bumpAt < BUMP_GAP) return;
    this.bumpAt = now;
    this.margin = Math.min(MARGIN_MAX, this.margin + MARGIN_STEP);
  }

  /** Off (paused, rewound, a jump): back to 1×, the window starts again. The margin stays: it
   *  belongs to the stream. */
  reset(): void {
    this.samples = [];
    this.rate = 1;
  }

  /** V111-38 `dt` s of steady play ending in a reading of `ahead` s: every CALM s in which the
   *  last step went unused take it off. */
  private wear(dt: number, ahead: number): void {
    if (this.margin <= 0) return;
    this.calm += dt;
    this.calmLow = Math.min(this.calmLow, ahead);
    if (this.calm < CALM) return;
    const unused = this.calmLow >= NEAR_DRY + MARGIN_STEP;
    this.calm = 0;
    this.calmLow = Infinity;
    if (unused) this.margin = Math.max(0, this.margin - MARGIN_STEP);
  }
}
