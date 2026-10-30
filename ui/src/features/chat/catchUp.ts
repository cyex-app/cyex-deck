// V108-N18 Catch up: where "New since you left" goes and what the "While you were away" card
// counts, from the lines still loaded and the moments the chat found. Pure. Runs once when
// you come back, never per line.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { Moment } from '../../ipc/bindings/Moment';
import { formatNumber } from '../../lib/format.ts';

type Line = Pick<ChatMessage, 'id' | 'ts' | 'flags' | 'notice' | 'highlight'>;

/** The most mentions or moments the card lists. */
export const PICKS = 3;

/** Away long enough? `mins` is the setting; `since` and `now` are unix ms. */
export function isAway(since: number, now: number, mins: number): boolean {
  return now - since >= Math.max(1, mins) * 60_000;
}

export interface Marker {
  /** The first line said after `since`; the divider goes before it. null: nothing new. */
  id: string | null;
  /** Index of `id` in `lines`. */
  at: number;
  /** The budget cut lines from the time you left: the oldest loaded line is already newer. */
  trimmed: boolean;
}

/** Walks back from the newest line while lines are newer than `since`. Lines trimmed by the
 *  scrollback budget are not there, so when the oldest loaded line is already newer the
 *  marker sits on it and `trimmed` says so. */
export function markerFor(lines: readonly Pick<ChatMessage, 'id' | 'ts'>[], since: number): Marker {
  let at = lines.length;
  while (at > 0 && lines[at - 1].ts > since) at--;
  if (at === lines.length) return { id: null, at: -1, trimmed: false };
  return { id: lines[at].id, at, trimmed: at === 0 };
}

export interface Summary {
  /** Lines since the marker. */
  total: number;
  mentions: number;
  highlights: number;
  subs: number;
  raids: number;
  /** Moments noticed after `since` (the chat's own list, not only loaded lines). */
  moments: number;
}

export interface Catch {
  summary: Summary;
  /** Newest mention lines, newest first, at most PICKS. */
  mentionIds: string[];
  /** Newest moments after `since`, newest first, at most PICKS. */
  moments: Moment[];
}

const MENTION_FLAGS = new Set(['mention', 'namesMe', 'repliesToMe']);

/** Counts the lines from `marker.at` on and the moments after `since`. */
export function catchOf(lines: readonly Line[], marker: Marker, moments: readonly Moment[], since: number): Catch {
  const s: Summary = { total: 0, mentions: 0, highlights: 0, subs: 0, raids: 0, moments: 0 };
  const mentionIds: string[] = [];
  if (marker.id !== null) {
    for (let i = marker.at; i < lines.length; i++) {
      const m = lines[i];
      if (m.flags.includes('own') || m.flags.includes('history')) continue;
      s.total++;
      const kind = m.notice?.kind;
      if (kind === 'sub' || kind === 'gift') s.subs++;
      else if (kind === 'raid') s.raids++;
      else if (m.highlight?.kind === 'mention' || m.flags.some((f) => MENTION_FLAGS.has(f))) {
        s.mentions++;
        mentionIds.push(m.id);
      } else if (m.highlight && !kind) s.highlights++;
    }
  }
  const found = moments.filter((x) => x.atMs > since);
  s.moments = found.length;
  return {
    summary: s,
    mentionIds: mentionIds.slice(-PICKS).reverse(),
    moments: found.slice(-PICKS).reverse(),
  };
}

/** Nothing worth a card. */
export function isQuiet(s: Summary): boolean {
  return s.mentions + s.highlights + s.subs + s.raids + s.moments === 0;
}

const count = (n: number, one: string, many: string) => `${formatNumber(n)} ${n === 1 ? one : many}`;

/** "2 mentions · 1 raid · 3 moments"; empty when quiet. */
export function summaryText(s: Summary): string {
  const parts: string[] = [];
  if (s.mentions) parts.push(count(s.mentions, 'mention', 'mentions'));
  if (s.highlights) parts.push(count(s.highlights, 'highlight', 'highlights'));
  if (s.subs) parts.push(count(s.subs, 'sub event', 'sub events'));
  if (s.raids) parts.push(count(s.raids, 'raid', 'raids'));
  if (s.moments) parts.push(count(s.moments, 'moment', 'moments'));
  return parts.join(' · ');
}

/** "7 min", "1 h 5 min". */
export function awayText(ms: number): string {
  const mins = Math.max(1, Math.round(ms / 60_000));
  if (mins < 60) return `${mins} min`;
  const h = Math.floor(mins / 60);
  const r = mins % 60;
  return r ? `${h} h ${r} min` : `${h} h`;
}

/** The first loaded line said at or after `atMs`: where a moment shows in the list. */
export function lineAt(lines: readonly Pick<ChatMessage, 'id' | 'ts'>[], atMs: number): string | null {
  let lo = 0;
  let hi = lines.length;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (lines[mid].ts < atMs) lo = mid + 1;
    else hi = mid;
  }
  return lo < lines.length ? lines[lo].id : null;
}
