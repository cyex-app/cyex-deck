// 1.43 The GIFs tab in the emote menu: when it shows, the cooldown line, send errors.

import type { GifInfo } from '../../ipc/bindings/GifInfo';
import type { GifSent } from '../../ipc/bindings/GifSent';

/** Only where this account can send one here right now; never a disabled tab. Rust
 * checks tier, broadcaster, channel and Shield Mode; emote-only can change any second. */
export function showGifTab(info: GifInfo | null | undefined, emoteOnly: boolean): boolean {
  return !!info?.canSend && !emoteOnly;
}

/** Whole seconds left until `until` (ms), rounded up; 0 once it passed. */
export function secsLeft(until: number, now: number): number {
  return Math.max(0, Math.ceil((until - now) / 1000));
}

export function cooldownText(secs: number): string {
  return `Next GIF in ${secs} s`;
}

/** What to say after a send; `null` when it went through or only a wait is due. */
export function sendError(r: GifSent): string | null {
  if (!r.error) return null;
  if (r.error === 'TEMPORARILY_UNAVAILABLE') return 'GIFs are unavailable right now.';
  if (r.waitSecs) return null;
  return 'Couldn’t send that GIF. Try again in a bit.';
}

/** Cooldowns per chat (account and channel), kept while the menu is closed. */
const until = new Map<string, number>();

export function cooldownUntil(key: string): number {
  return until.get(key) ?? 0;
}

/** After a send: Twitch's wait, if it gave one. Returns the new end (ms). */
export function startCooldown(key: string, r: GifSent, now: number): number {
  if (!r.waitSecs) return cooldownUntil(key);
  const end = now + r.waitSecs * 1000;
  until.set(key, end);
  return end;
}
