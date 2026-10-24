// V108-02 Where sign-ins open, remembered on this PC. Pure apart from the storage calls.

import type { Place } from '../../ipc/bindings/Place';

const KEY = 'cyex.signInPlace';

export function readPlace(): Place | null {
  try {
    const v = localStorage.getItem(KEY);
    return v === 'browser' || v === 'app' ? v : null;
  } catch {
    return null;
  }
}

export function savePlace(place: Place): void {
  try {
    localStorage.setItem(KEY, place);
  } catch {
    // Not remembered; the pick still holds until restart.
  }
}

/** The user's pick; else the browser, except for adding another account: the browser
 * would just sign in the account it already knows, so a fresh app window. */
export function placeFor(picked: Place | null, userId: string | null, accountCount: number): Place {
  if (picked) return picked;
  return userId === null && accountCount > 0 ? 'app' : 'browser';
}
