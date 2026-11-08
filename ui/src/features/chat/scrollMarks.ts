// V110-C55 Ticks on the chat's scrollbar: which lines get one, in which color, and where.
// Pure; ScrollMarks.svelte measures and draws.

/** What a line needs to be judged. */
export interface Markable {
  id: string;
  highlight?: { kind: string; color?: string };
  notice?: { kind: string };
}

/** A CSS variable name (`--red`) or a `#rrggbb[aa]` color. */
export interface MarkLook {
  color: string;
  /** Higher draws on top when ticks share a spot. */
  rank: number;
}

/** The highlight kinds, as the line's own look colors them (Line.svelte, `.hl-*`). */
const HIGHLIGHT: Record<string, MarkLook> = {
  mention: { color: '--accent-line', rank: 8 },
  rule: { color: '--red', rank: 7 },
  firstMessage: { color: '--green', rank: 5 },
  redeemed: { color: '--twitch', rank: 4 },
  sub: { color: '--twitch', rank: 4 },
  announcement: { color: '--twitch', rank: 4 },
  watchStreak: { color: '--cyan', rank: 3 },
  own: { color: '--accent-hover', rank: 2 },
};

/** Event lines without a highlight of their own: the twitch purple lines, mod actions, send errors. */
const NOTICE: Record<string, MarkLook> = {
  sub: HIGHLIGHT.sub,
  gift: HIGHLIGHT.sub,
  raid: HIGHLIGHT.sub,
  announcement: HIGHLIGHT.sub,
  bits: HIGHLIGHT.sub,
  milestone: HIGHLIGHT.sub,
  redemption: HIGHLIGHT.sub,
  moderation: { color: '--yellow', rank: 1 },
  error: { color: '--red', rank: 6 },
};

/** The look of this line's tick, or null for a plain line. A highlight's own color wins. */
export function markOf(m: Markable): MarkLook | null {
  const h = m.highlight;
  if (h) {
    const base = HIGHLIGHT[h.kind];
    if (h.color) return { color: h.color, rank: base?.rank ?? 7 };
    if (base) return base;
  }
  return (m.notice && NOTICE[m.notice.kind]) || null;
}

export interface Tick {
  id: string;
  /** Pixels from the top of the track. */
  y: number;
  color: string;
  rank: number;
}

/** A line that has a tick, and where it sits in the scrolled content. */
export interface Placed extends MarkLook {
  id: string;
  /** Middle of the line, in pixels of scroll content. */
  mid: number;
}

/** Places ticks on a track of `track` pixels for content `content` pixels high. Ticks closer than
 *  `gap` pixels share one place: the higher rank (then the newer line) stays. Ordered by rank,
 *  so drawing them in order leaves the important ones on top. */
export function place(lines: Placed[], content: number, track: number, gap = 3): Tick[] {
  if (content <= 0 || track <= 0) return [];
  const slots = new Map<number, Tick>();
  for (const l of lines) {
    const y = Math.min(track - 1, Math.max(0, (l.mid / content) * track));
    const slot = Math.round(y / gap);
    const held = slots.get(slot);
    if (!held || l.rank >= held.rank) slots.set(slot, { id: l.id, y, color: l.color, rank: l.rank });
  }
  return [...slots.values()].sort((a, b) => a.rank - b.rank || a.y - b.y);
}

/** The tick within `reach` pixels of `y`, the closest one. */
export function nearest(ticks: readonly Tick[], y: number, reach = 4): Tick | null {
  let best: Tick | null = null;
  let gap = reach + 1;
  for (const t of ticks) {
    const d = Math.abs(t.y - y);
    if (d < gap || (d === gap && best && t.rank > best.rank)) {
      best = t;
      gap = d;
    }
  }
  return best;
}
