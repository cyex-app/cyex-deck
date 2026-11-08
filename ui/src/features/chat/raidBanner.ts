// V110-U43 The raid banner's rules: when Join shows, the countdown, and what the pinned
// message does while a raid shows (folded to one line, then back the way it was).

/** Join shows only when the setting is on, a raid counts down, the chat is signed in and it
 * wasn't joined yet. Joining is never automatic. */
export function joinShows(setting: boolean, raidId: string | null, account: string | null, joined: string | null): boolean {
  return setting && !!raidId && !!account && joined !== raidId;
}

/** "1:05", "0:09"; "now" once the time is up. `goesAt` in unix seconds. */
export function raidLeft(goesAt: number, nowMs: number): string {
  const s = Math.ceil(goesAt - nowMs / 1000);
  if (!(s > 0)) return 'now';
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** "1 viewer", "1.234 viewers". */
export function viewersText(n: number, format: (n: number) => string): string {
  return `${format(n)} ${n === 1 ? 'viewer' : 'viewers'}`;
}

/** The pin's fold while raids come and go. `open`: unfolded. `before`: how it was when the
 * raid started (null when no raid shows). `hand`: folded or unfolded by hand during the raid,
 * so that choice stays. Hidden or closed pins are the event strip's and stay as they are. */
export interface PinFold {
  open: boolean;
  before: boolean | null;
  hand: boolean;
}

export const PIN_OPEN: PinFold = { open: true, before: null, hand: false };

/** A raid shows or stops showing. Same input twice changes nothing. */
export function raidFold(s: PinFold, raiding: boolean): PinFold {
  if (raiding && s.before === null) return { open: false, before: s.open, hand: false };
  if (!raiding && s.before !== null) return { open: s.hand ? s.open : s.before, before: null, hand: false };
  return s;
}

/** The pin is folded or unfolded by hand. */
export function handFold(s: PinFold): PinFold {
  return { open: !s.open, before: s.before, hand: s.before !== null };
}

/** A new pin starts open, folded right away while a raid shows. */
export function newPin(raiding: boolean): PinFold {
  return raidFold(PIN_OPEN, raiding);
}

/** Folded for the raid: the one-line form (no head, one line of text). */
export function raidSlim(s: PinFold): boolean {
  return s.before !== null && !s.hand && !s.open;
}
