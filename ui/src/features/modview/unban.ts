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
