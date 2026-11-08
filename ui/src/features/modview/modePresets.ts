// V110-M23 Typed lengths for the slow mode, followers-only and ad presets. Same limits as the
// Rust schema (crates/settings/src/schema/chat.rs), which drops anything outside them.
import { parseDuration } from '../../lib/format.ts';

export type Parsed = { value: number } | { error: string };

export const SLOW_RANGE = { min: 3, max: 120 };
export const FOLLOW_MAX_MINUTES = 129_600;
export const AD_LENGTHS = [30, 60, 90, 120, 150, 180];

/** Slow mode, in seconds: "30", "45s" or "2m". Twitch takes 3 s to 2 min. */
export function slowSecs(text: string): Parsed {
  const secs = parseDuration(text);
  if (secs === null) return { error: 'Try 30s or 2m.' };
  if (secs < SLOW_RANGE.min || secs > SLOW_RANGE.max) return { error: 'Twitch allows 3s to 2m.' };
  return { value: secs };
}

/** Followers-only age, in minutes: a bare number is minutes, else "30m", "1d", "2w", "3mo". */
export function followMinutes(text: string): Parsed {
  const t = text.trim().toLowerCase();
  const mo = /^(\d+)\s*mo$/.exec(t);
  const secs = mo ? Number(mo[1]) * 2_592_000 : /^\d+$/.test(t) ? Number(t) * 60 : parseDuration(t);
  if (secs === null || secs < 60) return { error: 'Try 30m, 1d, 2w or 3mo.' };
  if (secs % 60 !== 0) return { error: 'Whole minutes only.' };
  if (secs / 60 > FOLLOW_MAX_MINUTES) return { error: 'Twitch allows up to 3 months.' };
  return { value: secs / 60 };
}

/** Ad length, in seconds: Twitch runs 30 to 180 in steps of 30. */
export function adSecs(text: string): Parsed {
  const secs = parseDuration(text);
  if (secs === null || !AD_LENGTHS.includes(secs)) return { error: 'Use 30, 60, 90, 120, 150 or 180s.' };
  return { value: secs };
}

/** The list with `value` added: sorted, once, at most `max` long. */
export function withPreset(list: number[], value: number, max = 10): number[] {
  return [...new Set([...list, value])].sort((a, b) => a - b).slice(0, max);
}
