// V107-R21b Mass time out: a number plus a unit, checked against Twitch's 1 s .. 14 days.
// 0 means a ban, as before.
export type DurationUnit = 'seconds' | 'minutes' | 'hours' | 'days';

export const DURATION_UNITS: readonly { value: DurationUnit; label: string }[] = [
  { value: 'seconds', label: 'Seconds' },
  { value: 'minutes', label: 'Minutes' },
  { value: 'hours', label: 'Hours' },
  { value: 'days', label: 'Days' },
];

const PER: Record<DurationUnit, number> = { seconds: 1, minutes: 60, hours: 3600, days: 86400 };
export const MAX_TIMEOUT_SECS = 14 * 86400;

export function toSeconds(n: number, unit: DurationUnit): number {
  return Math.round(n * PER[unit]);
}

/** Seconds to time out for, or null for a ban (0 or not a number). */
export function timeoutSecs(n: number, unit: DurationUnit): number | null {
  const s = toSeconds(n, unit);
  return Number.isFinite(s) && s > 0 ? s : null;
}

/** Short inline error, or null when the duration is allowed (0 = ban is allowed). */
export function durationError(n: number, unit: DurationUnit): string | null {
  const s = timeoutSecs(n, unit);
  if (s !== null && s > MAX_TIMEOUT_SECS) return 'Twitch allows up to 14 days.';
  return null;
}
