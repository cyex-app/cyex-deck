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

/** The user's pick; else the browser, for every sign-in (adding another account too). */
export function placeFor(picked: Place | null): Place {
  return picked ?? 'browser';
}
