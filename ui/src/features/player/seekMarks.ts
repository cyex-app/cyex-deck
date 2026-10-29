// V108-N25 Marks on the seek bar (moments): which one a pointer is on.

export interface BarMark {
  id: number;
  /** Seconds from the bar's start. */
  t: number;
  text: string;
  kind: string;
}

/** The mark within `px` pixels of the pointer at `t` s on a bar of `duration` s over `width`
 *  px; the nearest when two are. null when none is near. */
export function markAt(marks: readonly BarMark[], t: number, duration: number, width: number, px = 6): BarMark | null {
  if (!marks.length || duration <= 0 || width <= 0) return null;
  const reach = (px / width) * duration;
  let best: BarMark | null = null;
  let gap = Infinity;
  for (const m of marks) {
    const d = Math.abs(m.t - t);
    if (d <= reach && d < gap) {
      best = m;
      gap = d;
    }
  }
  return best;
}
