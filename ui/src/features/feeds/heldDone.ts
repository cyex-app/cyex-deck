// V111-17 What a decided held message says, the same in chat, the mod view queue and the
// AutoMod feed: "Approved by Kuni", "Denied", "Expired". Pending: nothing (the buttons show).

import type { HeldStatus } from '../../ipc/bindings/HeldStatus';

export function heldDone(status: HeldStatus, by: string | null | undefined): string | null {
  const who = by ? ` by ${by}` : '';
  if (status === 'allowed') return `Approved${who}`;
  if (status === 'denied') return `Denied${who}`;
  if (status === 'expired') return 'Expired';
  return null;
}

/** s3-11 How long a click waits for Twitch's decision event before saying it didn't go through. */
export const HELD_WAIT_MS = 10_000;
