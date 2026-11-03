// V108-C35b How the hype train bar and the unlock card look: the fill's colours (a washed
// pastel half rainbow, gold for a Golden Kappa train), the level timer's span and the
// sparkles' paths. Pure, so it is tested without a DOM.

/** Twitch's Golden Kappa (24 h after a Golden Kappa train). Typed as "Kappa". */
export const GOLDEN_KAPPA_ID = '80393';

export type Look = 'pastel' | 'gold';

/** Three stops, left to right. The bar shows them washed out (see HypeTrain.svelte). */
export const PALETTE: Record<Look, readonly [string, string, string]> = {
  pastel: ['#ffbfa3', '#f5a9cf', '#b8a6ff'],
  gold: ['#ffe08a', '#f4c55a', '#e2a640'],
};

/** Sparkles are drawn at full strength, so they read on the washed fill. */
const SPARK: Record<Look, readonly string[]> = {
  pastel: ['#ffb08f', '#f58fc2', '#a48dff', '#ffd3a8'],
  gold: ['#ffd75e', '#f4b942', '#fff1b8', '#e8a12e'],
};

export function lookOf(t: { goldenKappa: boolean } | null | undefined): Look {
  return t?.goldenKappa ? 'gold' : 'pastel';
}

export function isGoldenKappa(id: string): boolean {
  return id === GOLDEN_KAPPA_ID;
}

/** `linear-gradient` for a look, laid along the whole bar. */
export function gradient(look: Look): string {
  const [a, b, c] = PALETTE[look];
  return `linear-gradient(90deg, ${a} 0%, ${b} 55%, ${c} 100%)`;
}

/** "45 %" of the level (the app writes percentages with a space). */
export function pctText(pct: number): string {
  return `${Math.max(0, Math.min(100, Math.floor(pct)))} %`;
}

/** A level runs 5 minutes on Twitch; the timer counts down from that. */
export const LEVEL_MS = 5 * 60_000;

/** What the timer bar is full at: a level's time, or more when Twitch gave more. */
export function levelSpan(leftMs: number): number {
  return Math.max(LEVEL_MS, leftMs);
}

/** How full the timer bar is, 0 to 1. */
export function timerFraction(leftMs: number, spanMs: number): number {
  if (!(spanMs > 0) || !(leftMs > 0)) return 0;
  return Math.min(1, leftMs / spanMs);
}

/** Milliseconds until `expiresAt` (RFC 3339); 0 once past or unknown. */
export function msLeft(expiresAt: string | null, nowMs: number): number {
  if (!expiresAt) return 0;
  const t = Date.parse(expiresAt);
  return Number.isNaN(t) ? 0 : Math.max(0, t - nowMs);
}

/** One sparkle: where it starts (in % of its layer), where it flies (px), when and how. */
export interface Spark {
  x: number;
  y: number;
  dx: number;
  dy: number;
  delay: number;
  size: number;
  color: string;
  /** A four-point star instead of a dot. */
  star: boolean;
}

/** burst: out of the bolt (start, level up); shower: along the bar (end); ring: round the
 * unlocked emote. Fixed paths, so every celebration looks the same and tests can hold it. */
export type SparkKind = 'burst' | 'shower' | 'ring';

export function sparks(kind: SparkKind, look: Look): Spark[] {
  const colors = SPARK[look];
  const color = (i: number) => colors[i % colors.length];
  if (kind === 'burst') {
    // A fan to the right and up/down of the bolt; the bar is short, so they stay low.
    const angles = [-70, -40, -15, 10, 35, 60, -100, 95];
    return angles.map((a, i) => {
      const r = 14 + (i % 3) * 5;
      const rad = (a * Math.PI) / 180;
      return { x: 0, y: 50, dx: Math.round(Math.cos(rad) * r), dy: Math.round(Math.sin(rad) * r * 0.7), delay: i * 18, size: 3 + (i % 2), color: color(i), star: i % 3 === 0 };
    });
  }
  if (kind === 'shower') {
    const xs = [7, 19, 30, 42, 53, 64, 76, 87, 95];
    return xs.map((x, i) => ({ x, y: 55 + (i % 2) * 15, dx: (i % 3) - 1, dy: -(10 + (i % 4) * 3), delay: (i * 47) % 400, size: 4 + (i % 3), color: color(i), star: i % 2 === 0 }));
  }
  const n = 8;
  return Array.from({ length: n }, (_, i) => {
    const rad = ((i / n) * 360 - 90) * (Math.PI / 180);
    const r = 34 + (i % 2) * 8;
    return { x: 50, y: 50, dx: Math.round(Math.cos(rad) * r), dy: Math.round(Math.sin(rad) * r), delay: 120 + i * 25, size: 4 + (i % 2) * 2, color: color(i), star: i % 2 === 0 };
  });
}

/** V108-C35b The unlock card's lines for one emote. */
export function unlockLines(e: { id: string; name: string }): { title: string; sub: string } {
  if (isGoldenKappa(e.id)) return { title: 'Golden Kappa unlocked', sub: 'Yours for 24 hours' };
  return { title: 'Emote unlocked', sub: e.name };
}

/** New emotes join the queue once; the one showing and those waiting aren't added again. */
export function enqueue<T extends { id: string }>(showing: T | null, queue: T[], incoming: T[]): T[] {
  const seen = new Set([...(showing ? [showing.id] : []), ...queue.map((e) => e.id)]);
  const out = [...queue];
  for (const e of incoming) {
    if (seen.has(e.id)) continue;
    seen.add(e.id);
    out.push(e);
  }
  // The Golden Kappa goes first: it only lasts a day.
  return out.sort((a, b) => Number(isGoldenKappa(b.id)) - Number(isGoldenKappa(a.id)));
}
