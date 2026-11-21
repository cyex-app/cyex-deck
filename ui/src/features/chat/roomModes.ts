// V112-27 Chat modes from the header chip: what the chip lists, who may change them, the
// lengths the menu offers, and the chat command each change sends (the same path as /slow
// and the rest, so Rust's role gate and Helix are not duplicated). Pure.

import type { Caps } from '../../ipc/bindings/Caps';
import type { RoomState } from '../../ipc/bindings/RoomState';
import { formatDuration, formatSecs } from '../../lib/format.ts';

export interface ModeChip {
  label: string;
  hint: string;
}

/** The chips for what is on, in the header's order. */
export function activeModes(s: Pick<RoomState, 'slow' | 'followersOnly' | 'subsOnly' | 'emoteOnly' | 'unique'>): ModeChip[] {
  const list: ModeChip[] = [];
  if (s.slow) list.push({ label: `Slow ${formatDuration(s.slow)}`, hint: 'One message per this long.' });
  if (s.followersOnly !== null)
    list.push({
      label: s.followersOnly ? `Followers ${formatDuration(s.followersOnly * 60)}` : 'Followers',
      hint: 'Only followers can chat.',
    });
  if (s.subsOnly) list.push({ label: 'Sub', hint: 'Only subscribers can chat.' });
  if (s.emoteOnly) list.push({ label: 'Emotes', hint: 'Emotes only.' });
  if (s.unique) list.push({ label: 'Unique', hint: 'Messages must be unique.' });
  return list;
}

/** Broadcaster, lead mod and mod: whoever the role gate gives caps. */
export const canSetModes = (caps: Caps | null | undefined): boolean => !!caps;

// V112-29 Slow mode and followers-only take a typed length instead of fixed steps.
// Slow mode: Twitch's API (Helix `slow_mode_wait_time`) and `/slow` only accept 3 to 120
// seconds, so the field stops at 2 minutes even though 5 minutes was asked for.
export const SLOW_MIN = 3;
export const SLOW_MAX = 120;
/** Followers-only: 0 (any follower) to 3 months of 30 days, in minutes. */
export const FOLLOW_MAX = 129_600;

export const SLOW_RANGE_NOTE = 'Slow mode goes from 3 seconds to 2 minutes.';
export const FOLLOW_RANGE_NOTE = 'Followers only goes from 0 minutes to 3 months.';

/** What the deck offers in the field before anything was used. */
export const MODE_DEFAULTS = { slow: 30, followers: 0 };

const UNIT_SECS: Record<string, number> = { s: 1, m: 60, h: 3600, d: 86_400, w: 604_800, mo: 2_592_000 };

/** "30", "30s", "2m", "1h", "1d", "1w", "3mo" as seconds; a bare number is `bare` ("s" or "m"). Null when unreadable. */
function lengthSecs(text: string, bare: 's' | 'm'): number | null {
  const m = /^(\d{1,9})\s*(s|m|h|d|w|mo)?$/.exec(text.trim().toLowerCase());
  if (!m) return null;
  return Number(m[1]) * UNIT_SECS[m[2] ?? bare];
}

export type Parsed = { value: number } | { error: string };

/** Slow mode seconds, 3 to 120. */
export function parseSlow(text: string): Parsed {
  const secs = lengthSecs(text, 's');
  return secs === null || secs < SLOW_MIN || secs > SLOW_MAX ? { error: SLOW_RANGE_NOTE } : { value: secs };
}

/** Followers-only minutes, 0 to 129600; whole minutes only. */
export function parseFollowers(text: string): Parsed {
  const secs = lengthSecs(text, 'm');
  if (secs === null || secs % 60 !== 0 || secs / 60 > FOLLOW_MAX) return { error: FOLLOW_RANGE_NOTE };
  return { value: secs / 60 };
}

/** The length as the field shows it: 120 -> "2m". */
export const slowText = (secs: number): string => (secs >= 60 && secs % 60 === 0 ? `${secs / 60}m` : `${secs}s`);
/** 0 -> "0", 60 -> "1h", 129600 -> "3mo". */
export const followText = (mins: number): string => (mins ? formatSecs(mins * 60) : '0');

/** The last length applied this session, for the field's start value. */
const last = { ...MODE_DEFAULTS };
export const lastModes = (): { slow: number; followers: number } => ({ ...last });
export function rememberMode(c: ModeChange): void {
  if (c.kind === 'slow' && c.secs) last.slow = c.secs;
  if (c.kind === 'followers' && c.mins !== null) last.followers = c.mins;
}

/** One change from the menu; `null` turns slow or followers-only off. */
export type ModeChange =
  | { kind: 'emote'; on: boolean }
  | { kind: 'subs'; on: boolean }
  | { kind: 'unique'; on: boolean }
  | { kind: 'slow'; secs: number | null }
  | { kind: 'followers'; mins: number | null };

/** The chat command for a change. */
export function modeCommand(c: ModeChange): string {
  switch (c.kind) {
    case 'emote':
      return c.on ? '/emoteonly' : '/emoteonlyoff';
    case 'subs':
      return c.on ? '/subscribers' : '/subscribersoff';
    case 'unique':
      return c.on ? '/uniquechat' : '/uniquechatoff';
    case 'slow':
      return c.secs === null ? '/slowoff' : `/slow ${c.secs}`;
    case 'followers':
      return c.mins === null ? '/followersoff' : c.mins === 0 ? '/followers' : `/followers ${c.mins}`;
  }
}

/** The state as it will be once Twitch accepts the change, shown at once; the real ROOMSTATE replaces it. */
export function withChange<S extends Pick<RoomState, 'slow' | 'followersOnly' | 'subsOnly' | 'emoteOnly' | 'unique'>>(s: S, c: ModeChange): S {
  switch (c.kind) {
    case 'emote':
      return { ...s, emoteOnly: c.on };
    case 'subs':
      return { ...s, subsOnly: c.on };
    case 'unique':
      return { ...s, unique: c.on };
    case 'slow':
      return { ...s, slow: c.secs ?? 0 };
    case 'followers':
      return { ...s, followersOnly: c.mins };
  }
}

/** V112-30 Which length a dialog asks for. */
export type LengthKind = 'slow' | 'followers';

/** What the dialog shows for each kind. */
export const LENGTH_ASK: Record<LengthKind, { title: string; label: string; placeholder: string; note: string }> = {
  slow: { title: 'Slow mode', label: 'Seconds', placeholder: '30', note: SLOW_RANGE_NOTE },
  followers: { title: 'Followers only', label: 'Time', placeholder: '0, 15m, 1h, 1d, 1w, 3mo', note: FOLLOW_RANGE_NOTE },
};

/** The dialog's start text: the last length used, or the default. */
export function lengthStart(kind: LengthKind, last: { slow: number; followers: number }): string {
  return kind === 'slow' ? String(last.slow) : followText(last.followers);
}

/** OK in the dialog: the change to send, or the range note to show with nothing sent. */
export function submitLength(kind: LengthKind, text: string): { change: ModeChange } | { error: string } {
  const p = kind === 'slow' ? parseSlow(text) : parseFollowers(text);
  if ('error' in p) return p;
  return { change: kind === 'slow' ? { kind: 'slow', secs: p.value } : { kind: 'followers', mins: p.value } };
}
