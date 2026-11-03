// V108-C36 The unban requests' small rules: when the list and the open request sit side by
// side (as whispers do), which request opens after one is resolved, and how Twitch's
// values read.

import { formatNumber } from '../../lib/format.ts';

/** Whispers' breakpoint: under it the list and the open request stack. */
export const NARROW = 480;

/** Two panes only popped out and wide enough; docked, always one at a time. */
export const isSplit = (popped: boolean, width: number) => popped && width >= NARROW;

/** The request to show once `id` is gone: the next one down, else the one above, else none. */
export function nextAfter(ids: string[], id: string): string | null {
  const i = ids.indexOf(id);
  if (i < 0) return ids[0] ?? null;
  return ids[i + 1] ?? ids[i - 1] ?? null;
}

const STATUS: Record<string, string> = {
  PENDING: 'Pending',
  APPROVED: 'Approved',
  DENIED: 'Denied',
  ACKNOWLEDGED: 'Closed',
  CANCELED: 'Withdrawn',
};

/** Twitch's request status in words; an unknown one as Twitch wrote it, in sentence case. */
export function statusText(status: string): string {
  const s = status.toUpperCase();
  return STATUS[s] ?? (s ? s[0] + s.slice(1).toLowerCase() : '');
}

/** Twitch's mod logs count messages up to 1000. */
export const messageCount = (n: number) => (n >= 1000 ? '1.000+' : formatNumber(n));

/** Who banned them, unless that is hidden (the header's eye). */
export function bannedBy(by: string | null | undefined, hidden: boolean): string {
  if (hidden) return 'Hidden';
  return by?.trim() || 'Unknown';
}

// V108-C36b The redesign's rules: list order, keys, the tab counts and the ban's chat line.

/** Requests by when they were sent, newest first unless `desc` is off; unreadable times last. */
export function orderRequests<T extends { createdAt: string }>(rows: readonly T[], desc: boolean): T[] {
  const t = (r: T) => {
    const v = Date.parse(r.createdAt);
    return Number.isNaN(v) ? null : v;
  };
  return [...rows].sort((a, b) => {
    const x = t(a);
    const y = t(b);
    if (x === null || y === null) return (x === null ? 1 : 0) - (y === null ? 1 : 0);
    return desc ? y - x : x - y;
  });
}

/** Up and down in the list stop at the ends (no wrap); Home and End jump. `null`: not a list key. */
export function listMove(key: string, at: number, n: number): number | null {
  if (!n) return null;
  switch (key) {
    case 'ArrowDown':
      return Math.min(at + 1, n - 1);
    case 'ArrowUp':
      return Math.max(at - 1, 0);
    case 'Home':
      return 0;
    case 'End':
      return n - 1;
    default:
      return null;
  }
}

/** Mod comments' count as Twitch shows it: this channel's, then "+n" shared from others. */
export function commentCount(comments: readonly { shared: boolean }[]): string {
  const shared = comments.filter((c) => c.shared).length;
  const own = formatNumber(comments.length - shared);
  return shared ? `${own} +${formatNumber(shared)}` : own;
}

/** The ban as the chat log's last line: "Kuni banned lumen_fox", "lumen_fox was timed out"
 *  when the eye hides who did it (or Twitch didn't say). */
export function banLine(ban: { by: string | null; until: string | null }, name: string, hidden: boolean): string {
  const what = ban.until ? 'timed out' : 'banned';
  const by = ban.by?.trim();
  return by && !hidden ? `${by} ${what} ${name}` : `${name} was ${what}`;
}
