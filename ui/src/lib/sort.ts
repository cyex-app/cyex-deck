// V107-Mr One sort for every list: a key (Time, A–Z, …) and a direction (↑ ascending,
// ↓ descending). Pure, so SortControl and the lists share it under the UI tests.

/** One key a list sorts by. */
export interface SortKey<K extends string = string> {
  value: K;
  label: string;
  /** The direction a fresh pick starts in (Time: newest first). */
  desc: boolean;
  /** What each direction gives, for the button's tip: [ascending, descending]. */
  words: readonly [string, string];
}

export interface Sort<K extends string = string> {
  key: K;
  desc: boolean;
}

const collator = new Intl.Collator(undefined, { numeric: true, sensitivity: 'base' });

/** Names, ignoring case and accents, numbers by value. */
export const byText = (a: string, b: string): number => collator.compare(a, b);

/** By a number (a time in ms, a count); rows with none (or 0) go last either way. */
export function byNumber<T>(get: (t: T) => number | null | undefined) {
  return (a: T, b: T, desc: boolean): number => {
    const x = get(a) || 0;
    const y = get(b) || 0;
    if (x && y) return x - y;
    // `sortRows` flips the result for `desc`; flip it back so empty rows stay last.
    return (desc ? -1 : 1) * ((x ? -1 : 0) + (y ? 1 : 0));
  };
}

/** The usual keys. */
export const TIME: SortKey<'time'> = { value: 'time', label: 'Time', desc: true, words: ['Oldest first', 'Newest first'] };
export const NAME: SortKey<'name'> = { value: 'name', label: 'A–Z', desc: false, words: ['A to Z', 'Z to A'] };

/** `sort` on `key`, starting in that key's own direction; the same key keeps its direction. */
export function pickKey<K extends string>(keys: readonly SortKey<K>[], sort: Sort<K>, key: K): Sort<K> {
  if (key === sort.key) return sort;
  return { key, desc: keys.find((k) => k.value === key)?.desc ?? false };
}

/** The other direction. */
export const flip = <K extends string>(sort: Sort<K>): Sort<K> => ({ key: sort.key, desc: !sort.desc });

/** The words for where `sort` is now: "Newest first". */
export function sortWords<K extends string>(keys: readonly SortKey<K>[], sort: Sort<K>): string {
  const k = keys.find((x) => x.value === sort.key);
  return k ? k.words[sort.desc ? 1 : 0] : sort.desc ? 'Descending' : 'Ascending';
}

/** `rows` in order: `by[key]` ascending, flipped when `desc`; `tie` (always ascending) settles
 *  equal rows. A comparator that keeps some rows last both ways takes `desc` (`byNumber`). */
export function sortRows<T, K extends string>(
  rows: readonly T[],
  sort: Sort<K>,
  by: Record<K, (a: T, b: T, desc: boolean) => number>,
  tie?: (a: T, b: T) => number,
): T[] {
  const cmp = by[sort.key];
  const dir = sort.desc ? -1 : 1;
  return [...rows].sort((a, b) => dir * cmp(a, b, sort.desc) || (tie ? tie(a, b) : 0));
}
