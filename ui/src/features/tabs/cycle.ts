// Moving through tabs in the bar's order: Ctrl+Tab wraps around at the ends, the mouse
// wheel over the tab bar wraps too (V107-H15).

import { groupAt } from './zones.ts';

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

/** V108-F07 Where a tab goes when moved `by` places along the bar: the tab to put it before
 * (null = the end) and the group it lands in, or null at an end of the bar. */
export function moveSlot(
  list: { id: string; group: string | null }[],
  id: string,
  by: number,
): { before: string | null; group: string | null } | null {
  const from = list.findIndex((t) => t.id === id);
  if (from < 0 || !by) return null;
  const to = from + by;
  if (to < 0 || to >= list.length) return null;
  const rest = list.filter((t) => t.id !== id);
  const [prev, next] = [rest[to - 1], rest[to]];
  return { before: next?.id ?? null, group: groupAt(prev, next, list[from].group) };
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
