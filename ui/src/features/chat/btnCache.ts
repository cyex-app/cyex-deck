// V107-My What the chat box's side buttons last looked like per channel, so a tab switch
// draws them at once instead of popping in once the room has opened.

import { CappedMap } from '../../lib/capped.ts';

export interface BtnMemo {
  /** Bits picker was shown: `false` once Twitch said cheering is off here. */
  bits?: boolean;
  /** Channel points button was hidden (no points, nothing to redeem). */
  pointsAway?: boolean;
  /** The channel's own points icon. */
  pointsIcon?: string | null;
  pointsName?: string;
}

const memo = new CappedMap<string, BtnMemo>(200);

export function btnMemo(login: string): BtnMemo {
  return memo.get(login.toLowerCase()) ?? {};
}

export function remember(login: string, patch: BtnMemo): void {
  const key = login.toLowerCase();
  const next = { ...memo.get(key), ...patch };
  memo.set(key, next);
}
