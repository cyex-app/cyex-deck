// V109-F79 What this window knows of each account's standing per channel, while it runs:
// a chat box remounted by a tab switch, or the chat closed and opened again, starts from it
// at once. The app keeps the same per (account, channel) (`chat_standing_known`), which a
// pop-out or a new window starts from.

import type { KnownStanding } from '../../ipc/bindings/KnownStanding';
import { CappedMap } from '../../lib/capped.ts';

// V110-F84 Capped: every account and channel opened in a long session adds one.
const kept = new CappedMap<string, KnownStanding>(500);

export const keyOf = (account: string, channelId: string): string => `${account}:${channelId}`;

export function cached(key: string): KnownStanding | null {
  return kept.get(key) ?? null;
}

export function keep(key: string, k: KnownStanding): void {
  kept.set(key, k);
}
