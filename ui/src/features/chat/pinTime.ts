// V110-B5 When a pinned message was sent, for the banner's head: "14:03" the same day,
// "2026/11/08 14:03" before that, nothing when Twitch didn't say (ts 0).
import { formatDate, formatTime } from '../../lib/format.ts';

export function pinSent(ts: number, now = Date.now()): string {
  if (!(ts > 0)) return '';
  const d = new Date(ts);
  if (Number.isNaN(d.getTime())) return '';
  const same = formatDate(d) === formatDate(new Date(now));
  return same ? formatTime(d) : `${formatDate(d)} ${formatTime(d)}`;
}
