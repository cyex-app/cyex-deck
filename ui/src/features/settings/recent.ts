// V107-Mo The last settings searches, newest first. Only ones that went somewhere count
// (a result picked, or a pause with results); kept in local settings (general.recentSearches).

export const MAX_RECENT = 5;
/** How long a search with results has to sit still to count. */
export const PAUSE_MS = 1000;
const MAX_CHARS = 80;

// By code point: cutting an emoji in half would leave a lone surrogate the settings file refuses.
const clean = (q: string) => Array.from(q.split(/\s+/).filter(Boolean).join(' ')).slice(0, MAX_CHARS).join('');
const same = (a: string, b: string) => a.toLowerCase() === b.toLowerCase();

/** `list` with `query` in front: trimmed, no repeat in any case, at most 5. Empty adds nothing. */
export function addRecent(list: readonly string[], query: string): string[] {
  const q = clean(query);
  if (!q) return [...list];
  return [q, ...list.filter((s) => !same(s, q))].slice(0, MAX_RECENT);
}

export function removeRecent(list: readonly string[], query: string): string[] {
  return list.filter((s) => !same(s, query));
}

/** One is the other typed on or cut back ("fo" and "font"). */
const refines = (a: string, b: string) => {
  const [x, y] = [a.toLowerCase(), b.toLowerCase()];
  return x !== y && (x.startsWith(y) || y.startsWith(x));
};

/**
 * One search counts once: `earlier` is what this search already added after a pause; typing
 * on from it ("fo" then "font") takes its place, a different search doesn't. Null when nothing
 * changes, so there's nothing to save.
 */
export function countSearch(list: readonly string[], query: string, earlier = ''): string[] | null {
  const q = clean(query);
  if (!q) return null;
  const e = clean(earlier);
  const base = e && refines(e, q) ? removeRecent(list, e) : list;
  const next = addRecent(base, q);
  return next.length === list.length && next.every((s, i) => s === list[i]) ? null : next;
}
