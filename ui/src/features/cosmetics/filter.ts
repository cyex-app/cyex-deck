// V1051-03 Search, chips and sort for the cosmetics menu. Pure.

import { formatNumber } from '../../lib/format.ts';
import { NAME, type Sort, TIME } from '../../lib/sort.ts';
import { rank, RANK } from '../chat/completion.ts';

/** V107-Mr Time or A–Z, either way round (the shared SortControl); newest first to start. */
export const COSMETIC_SORTS = [TIME, NAME];
export type CosmeticSort = Sort<'time' | 'name'>;

export type Chip = 'animated' | 'static';
export interface Query {
  text: string;
  chips: ReadonlySet<Chip>;
  sort: CosmeticSort;
}

interface Row {
  name: string;
  animated: boolean;
  added: number;
}

type Kind = 'paint' | 'badge';
/** The sort chosen per pane, kept for the session (the menu and the pop-up share it). */
const remembered: Partial<Record<Kind, Pick<Query, 'sort'>>> = {};
export const rememberSort = (kind: Kind, q: Query) => void (remembered[kind] = { sort: q.sort });

export const emptyQuery = (kind?: Kind): Query => ({
  text: '',
  chips: new Set(),
  sort: { key: 'time', desc: true },
  ...(kind && remembered[kind]),
});

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/** Time or name, either way round; ties go by name, A to Z. */
export function sorter<T extends { name: string; added: number }>(sort: CosmeticSort) {
  const dir = sort.desc ? -1 : 1;
  return sort.key === 'name'
    ? (a: T, b: T) => dir * collator.compare(a.name, b.name)
    : (a: T, b: T) => dir * (a.added - b.added) || collator.compare(a.name, b.name);
}

function keep(r: Row, chips: ReadonlySet<Chip>): boolean {
  // Animated and Static together means both.
  if (chips.has('animated') !== chips.has('static') && r.animated !== chips.has('animated')) return false;
  return true;
}

export function filterItems<T extends Row>(items: readonly T[], q: Query): T[] {
  const text = q.text.trim();
  const by = sorter<T>(q.sort);
  const rows = items.filter((r) => keep(r, q.chips));
  if (!text) return rows.sort(by);
  const scored: { r: T; rank: number; close: number }[] = [];
  for (const r of rows) {
    const s = rank(r.name, text);
    if (s.rank > RANK.none) scored.push({ r, rank: s.rank, close: s.close });
  }
  scored.sort((a, b) => b.rank - a.rank || b.close - a.close || by(a.r, b.r));
  return scored.map((s) => s.r);
}

/** V1051-03 "3 of 2.000" while a search or chip narrows the list, else just the total. */
export function countLabel(shown: number, total: number): string {
  return shown === total ? formatNumber(total) : `${formatNumber(shown)} of ${formatNumber(total)}`;
}
