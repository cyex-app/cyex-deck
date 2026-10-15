// V1051-03 Search, chips and sort for the cosmetics menu. Pure.

import { rank, RANK } from '../chat/completion.ts';

export type Chip = 'animated' | 'static' | 'usable' | 'new';
export interface Query {
  text: string;
  chips: ReadonlySet<Chip>;
  sort: 'newest' | 'az';
}
export const NEW_DAYS = 30;
const DAY = 86_400_000;

interface Row {
  name: string;
  animated: boolean;
  usable: boolean;
  added: number;
}

export const emptyQuery = (): Query => ({ text: '', chips: new Set(), sort: 'newest' });

function keep(r: Row, chips: ReadonlySet<Chip>, now: number): boolean {
  // Animated and Static together means both.
  if (chips.has('animated') !== chips.has('static') && r.animated !== chips.has('animated')) return false;
  if (chips.has('usable') && !r.usable) return false;
  if (chips.has('new') && r.added < now - NEW_DAYS * DAY) return false;
  return true;
}

export function filterItems<T extends Row>(items: readonly T[], q: Query, now: number): T[] {
  const text = q.text.trim();
  const by = q.sort === 'az' ? (a: T, b: T) => a.name.localeCompare(b.name) : (a: T, b: T) => b.added - a.added;
  const rows = items.filter((r) => keep(r, q.chips, now));
  if (!text) return rows.sort(by);
  const scored: { r: T; rank: number; close: number }[] = [];
  for (const r of rows) {
    const s = rank(r.name, text);
    if (s.rank > RANK.none) scored.push({ r, rank: s.rank, close: s.close });
  }
  scored.sort((a, b) => b.rank - a.rank || b.close - a.close || by(a.r, b.r));
  return scored.map((s) => s.r);
}
