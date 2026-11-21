// V112-27 Chat modes from the header chip: what the chip lists, who may change them, the
// lengths the menu offers, and the chat command each change sends (the same path as /slow
// and the rest, so Rust's role gate and Helix are not duplicated). Pure.

import type { Caps } from '../../ipc/bindings/Caps';
import type { RoomState } from '../../ipc/bindings/RoomState';
import { formatDuration } from '../../lib/format.ts';

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

/** Slow mode lengths, seconds. */
export const SLOW_STEPS: { secs: number; label: string }[] = [
  { secs: 3, label: '3s' },
  { secs: 5, label: '5s' },
  { secs: 10, label: '10s' },
  { secs: 30, label: '30s' },
  { secs: 60, label: '1m' },
  { secs: 120, label: '2m' },
  { secs: 300, label: '5m' },
];

/** Followers-only lengths, minutes (0 = any follower). */
export const FOLLOW_STEPS: { mins: number; label: string }[] = [
  { mins: 0, label: 'Any follower' },
  { mins: 10, label: '10m' },
  { mins: 30, label: '30m' },
  { mins: 60, label: '1h' },
  { mins: 1440, label: '1d' },
  { mins: 10_080, label: '1w' },
  { mins: 43_200, label: '1mo' },
  { mins: 129_600, label: '3mo' },
];

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
