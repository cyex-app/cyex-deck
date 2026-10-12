// Moving through tabs in the bar's order: Ctrl+Tab wraps around at the ends, the mouse
// wheel over the tab bar stops there (V102-04).

/** The tab `by` steps from `current`, or null when that's past an end (and `wrap` is off).
 * From no tab (home, discover), forward is the first tab and back the last. */
export function stepTab(ids: string[], current: string | null, by: number, wrap: boolean): string | null {
  const n = ids.length;
  if (!n || !by) return null;
  const at = current ? ids.indexOf(current) : -1;
  if (at < 0) return by > 0 ? ids[0] : ids[n - 1];
  const next = at + by;
  if (wrap) return ids[((next % n) + n) % n];
  return next < 0 || next >= n ? null : ids[next];
}

/** A mouse wheel notch is one tab. A touchpad's small deltas add up to one per 100 px, so
 * a swipe doesn't race through the bar. */
const NOTCH = 50;
const PER_STEP = 100;
/** A pause this long starts the count over. */
const IDLE_MS = 300;

export class WheelSteps {
  private sum = 0;
  private last = 0;

  /** Steps for one wheel event: +1 down (next), -1 up (previous), or 0. `lines`: the event
   * counts lines or pages, as some mice do. */
  step(delta: number, lines: boolean, now: number): number {
    if (now - this.last > IDLE_MS) this.sum = 0;
    this.last = now;
    if (!delta) return 0;
    if (lines || Math.abs(delta) >= NOTCH) {
      this.sum = 0;
      return Math.sign(delta);
    }
    this.sum += delta;
    if (Math.abs(this.sum) < PER_STEP) return 0;
    const s = Math.sign(this.sum);
    this.sum = 0;
    return s;
  }
}
