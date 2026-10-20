// V107-R22 A typed timeout length: "10s", "7m", "3h", "1d", "1h 30m" or bare seconds.
// Twitch takes 1 s to 14 days.

import { parseDuration } from '../../lib/format.ts';

export const TIMEOUT_MAX = 1_209_600;

/** Seconds, or the short line saying why not. */
export function timeoutSecs(text: string): { secs: number } | { error: string } {
  const secs = parseDuration(text);
  if (secs === null || secs < 1) return { error: 'Try 10s, 7m, 3h or 1d.' };
  if (secs > TIMEOUT_MAX) return { error: 'Twitch allows up to 14 days.' };
  return { secs };
}
