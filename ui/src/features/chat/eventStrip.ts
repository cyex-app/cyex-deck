// V103-04 The pinned message, poll and prediction above a chat can be hidden into one
// "N Events" button at the top; a click opens them all again. Close still removes one.
// Rules: only an item that's still going can be hidden, and it counts while it goes on.
// Once it ends (pin taken down or run out, poll over, prediction settled) it leaves the
// count and stays closed, so its result doesn't pop up. While the button shows, new items
// join it. Kept per channel, so a chat that remounts keeps what you hid.

import type { Pinned } from '../../ipc/bindings/Pinned';
import type { VotesUpdate } from '../../ipc/bindings/VotesUpdate';

/** One item above the chat. `key`: kind and Twitch id. `live`: still going. */
export interface ChatEvent {
  key: string;
  live: boolean;
}

export interface StripState {
  /** In the button. */
  readonly hidden: ReadonlySet<string>;
  /** Closed for good, or ended while hidden. */
  readonly closed: ReadonlySet<string>;
  /** On screen last time, to tell new items. */
  readonly seen: ReadonlySet<string>;
}

export const EMPTY_STRIP: StripState = { hidden: new Set(), closed: new Set(), seen: new Set() };

/** Closed keys kept per channel; older ones won't come back anyway. */
const MAX_CLOSED = 30;

/** Prediction states that still wait for a result. */
const PREDICTION_LIVE = ['ACTIVE', 'LOCKED', 'RESOLVE_PENDING'];

export const pinKey = (id: string) => `pin:${id}`;
export const pollKey = (id: string) => `poll:${id}`;
export const predictionKey = (id: string) => `prediction:${id}`;

export const pinLive = (pin: Pinned, now: number) => !pin.endsAt || !(Date.parse(pin.endsAt) <= now);
export const pollLive = (status: string) => status === 'ACTIVE';
export const predictionLive = (status: string) => PREDICTION_LIVE.includes(status);

/** What the chat has above its messages right now. */
export function chatEvents(pin: Pinned | null, votes: VotesUpdate | undefined, now: number): ChatEvent[] {
  const out: ChatEvent[] = [];
  if (pin) out.push({ key: pinKey(pin.id), live: pinLive(pin, now) });
  if (votes?.poll) out.push({ key: pollKey(votes.poll.id), live: pollLive(votes.poll.status) });
  const p = votes?.prediction;
  if (p) out.push({ key: predictionKey(p.id), live: predictionLive(p.status) });
  return out;
}

/** A chat that opens again (its tab came back) reads its pin a moment later. Until then a
 * hidden pin isn't gone, just not known yet, so it stays in the button. */
export function withUnreadPin(events: ChatEvent[], s: StripState, pinKnown: boolean): ChatEvent[] {
  if (pinKnown) return events;
  const waiting = [...s.hidden].filter((k) => k.startsWith(pinKey('')) && !events.some((e) => e.key === k));
  return [...events, ...waiting.map((key) => ({ key, live: true }))];
}

/** Brings the state up to date with what the chat has now. Returns `s` itself when
 * nothing changed. */
export function settle(s: StripState, events: readonly ChatEvent[]): StripState {
  const joining = s.hidden.size > 0;
  const hidden = new Set<string>();
  const closed = new Set(s.closed);
  for (const e of events) {
    if (s.hidden.has(e.key)) {
      if (e.live) hidden.add(e.key);
      else closed.add(e.key);
    } else if (joining && e.live && !s.seen.has(e.key) && !closed.has(e.key)) {
      hidden.add(e.key);
    }
  }
  const seen = new Set(events.map((e) => e.key));
  if (same(hidden, s.hidden) && same(closed, s.closed) && same(seen, s.seen)) return s;
  return { hidden, closed: trim(closed), seen };
}

/** Only what's still going can be hidden. */
export function hide(s: StripState, e: ChatEvent): StripState {
  if (!e.live || s.hidden.has(e.key)) return s;
  return { ...s, hidden: new Set([...s.hidden, e.key]) };
}

export function close(s: StripState, key: string): StripState {
  const hidden = new Set(s.hidden);
  hidden.delete(key);
  return { ...s, hidden, closed: trim(new Set([...s.closed, key])) };
}

/** The button was clicked: everything in it shows again. */
export function reopen(s: StripState): StripState {
  return s.hidden.size ? { ...s, hidden: new Set() } : s;
}

export function shows(s: StripState, key: string): boolean {
  return !s.hidden.has(key) && !s.closed.has(key);
}

/** "1 Event", "2 Events". */
export function eventsLabel(n: number): string {
  return `${n} ${n === 1 ? 'Event' : 'Events'}`;
}

function same(a: ReadonlySet<string>, b: ReadonlySet<string>): boolean {
  if (a.size !== b.size) return false;
  for (const k of a) if (!b.has(k)) return false;
  return true;
}

function trim(closed: Set<string>): Set<string> {
  if (closed.size <= MAX_CLOSED) return closed;
  return new Set([...closed].slice(-MAX_CLOSED));
}
