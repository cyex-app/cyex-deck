import type { Pinned } from '../../ipc/bindings/Pinned';

/** V106-19 How long a pin that ends has left. `null` for pins without an end. */
export interface PinSpan {
  /** Ms until the end (0 once over). */
  left: number;
  /** Share of the pin's length still to go, 0..1. */
  frac: number;
}

/** `seenAt` stands in for the start when Twitch sent none (or it's unreadable). */
export function pinSpan(pin: Pick<Pinned, 'startsAt' | 'endsAt'>, seenAt: number, now: number): PinSpan | null {
  if (!pin.endsAt) return null;
  const end = Date.parse(pin.endsAt);
  if (Number.isNaN(end)) return null;
  const parsed = pin.startsAt ? Date.parse(pin.startsAt) : NaN;
  const start = Number.isNaN(parsed) ? seenAt : parsed;
  const left = Math.max(0, end - now);
  const total = Math.max(end - start, left);
  return { left, frac: total > 0 ? Math.min(1, left / total) : 0 };
}
