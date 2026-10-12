// V104-01, V104-02 The Mentions tab is a feed tab (F4) named "Mentions": mentions that say
// "Show in Mentions" are copied there, global ones marked as such.

import { formatDate, formatTime } from '../../lib/format.ts';

export const MENTIONS_FEED = 'Mentions';

/** Rust keys feeds by their trimmed, lowercase name. */
export function isMentionsFeed(name: string | null | undefined): boolean {
  return name?.trim().toLowerCase() === MENTIONS_FEED.toLowerCase();
}

/** When a feed line was said (3.1, 3.2): the date when it isn't today, the time unless the
 * line shows it already (chat timestamps on). Empty when there's nothing to add. */
export function whenText(ts: number, stamped: boolean, now = Date.now()): string {
  const d = new Date(ts);
  const today = formatDate(d) === formatDate(new Date(now));
  if (today) return stamped ? '' : formatTime(d);
  return stamped ? formatDate(d) : `${formatDate(d)} ${formatTime(d)}`;
}
