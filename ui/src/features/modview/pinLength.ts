// V107 A custom pin length. Twitch takes any whole second from 30 s to 30 min (tested
// 2026/10/19: 31, 777 and 1.799 s pin; 29 and 1.801 s are refused), or no end at all.

import { parseDuration } from '../../lib/format.ts';

export const PIN_MIN = 30;
export const PIN_MAX = 1800;

/** "12" (minutes), "2:30" (m:ss) or "90s" / "1m 30s", clamped to 30 s..30 min. Null if unreadable. */
export function pinCustomSecs(text: string): number | null {
  const t = text.trim().toLowerCase();
  if (!t) return null;
  let secs: number | null;
  const clock = /^(\d{1,2}):([0-5]\d)$/.exec(t);
  if (clock) secs = Number(clock[1]) * 60 + Number(clock[2]);
  else if (/^\d+([.,]\d+)?$/.test(t)) secs = Math.round(Number(t.replace(',', '.')) * 60);
  else secs = parseDuration(t);
  if (!secs || secs <= 0) return null;
  return Math.min(PIN_MAX, Math.max(PIN_MIN, secs));
}
