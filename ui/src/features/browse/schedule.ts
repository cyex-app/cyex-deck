// V108-N03 Schedule text for Home's offline follows and About. Pure, so it's tested
// without Svelte. Times are local, dates yyyy/mm/dd, 24 hours.

import type { NextStream } from '../../ipc/bindings/NextStream';
import type { ScheduleSegment } from '../../ipc/bindings/ScheduleSegment';
import { formatDate, formatTime } from '../../lib/format.ts';

/** "2026/10/30 20:00" */
export function when(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? '' : `${formatDate(d)} ${formatTime(d)}`;
}

/** "next: 2026/10/30 20:00", empty when there is none or it has started. */
export function nextText(next: NextStream | undefined, now = Date.now()): string {
  if (!next) return '';
  const start = Date.parse(next.start);
  if (!start || start <= now) return '';
  return `next: ${when(next.start)}`;
}

/** A schedule line's time: "2026/10/30 20:00" and, with an end, "–22:00" (or the end's date
 *  when it falls on another day). */
export function segmentTime(s: Pick<ScheduleSegment, 'start' | 'end'>): string {
  const from = when(s.start);
  if (!from || !s.end) return from;
  const a = new Date(s.start);
  const b = new Date(s.end);
  if (Number.isNaN(b.getTime()) || b <= a) return from;
  return `${from}–${formatDate(a) === formatDate(b) ? formatTime(b) : when(s.end)}`;
}
