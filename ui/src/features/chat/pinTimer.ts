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

/** V107 The time a pin has left, rounded up to the second: "4:59", "0:05", "1:02:03". */
export function pinLeft(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const h = Math.floor(total / 3600);
  const m = Math.floor((total % 3600) / 60);
  const s = String(total % 60).padStart(2, '0');
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${s}` : `${m}:${s}`;
}
