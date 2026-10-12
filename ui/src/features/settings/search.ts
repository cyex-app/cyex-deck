// V103-13 Settings search over the registry. Ranked like match-sorter (Kent C. Dodds) with
// the emote completion's own helper (V101-16): an entry ranks by its best field, a tie goes
// to the earlier field, then to the order of the list. Per field, as match-sorter's
// `threshold` and `maxRanking`: hints only count from "contains" up, and a page or group
// name counts as "contains" at most, so a setting's own name beats the page it's on.
// Several words: the whole phrase first, then entries that match every word somewhere.

import { rank, RANK } from '../chat/completion.ts';
import type { Entry } from './registry/entry';
import type { Page } from './registry/pages';

interface Field {
  texts: (e: Entry, p: Page) => string[];
  min: number;
  max: number;
}

/** Searched fields, best first. */
const FIELDS: Field[] = [
  { texts: (e) => [e.label], min: RANK.letters, max: RANK.caseEqual },
  { texts: (e) => e.keywords ?? [], min: RANK.letters, max: RANK.caseEqual },
  { texts: (e) => (e.description ? [e.description] : []), min: RANK.contains, max: RANK.caseEqual },
  { texts: (_, p) => [p.label], min: RANK.contains, max: RANK.contains },
  { texts: (_, p) => [p.group], min: RANK.contains, max: RANK.contains },
];

export interface Doc {
  entry: Entry;
  page: Page;
  /** Folded texts per field, in `FIELDS` order. */
  fields: string[][];
  /** Place in the list: page order, then entry order. */
  order: number;
}

export interface Hit {
  entry: Entry;
  page: Page;
}

interface Score {
  rank: number;
  close: number;
  field: number;
}

const NONE: Score = { rank: RANK.none, close: 0, field: Infinity };

/** Accents off and spaces folded, like match-sorter. */
export function fold(text: string): string {
  return text.normalize('NFD').replace(/\p{M}/gu, '').replace(/\s+/g, ' ').trim();
}

/** Every entry whose page exists, ready to search. */
export function buildIndex(entries: readonly Entry[], pages: readonly Page[]): Doc[] {
  const docs: Doc[] = [];
  entries.forEach((entry, i) => {
    const at = pages.findIndex((p) => p.id === entry.page);
    if (at < 0) return;
    const page = pages[at];
    docs.push({ entry, page, fields: FIELDS.map((f) => f.texts(entry, page).map(fold)), order: at * 100_000 + i });
  });
  return docs;
}

/** The best field for `term`. A phrase of several words counts from "contains" up:
 * letters in order across spaces match too much. */
function best(doc: Doc, term: string, phrase: boolean): Score {
  let top = NONE;
  doc.fields.forEach((texts, field) => {
    const { min, max } = FIELDS[field];
    for (const text of texts) {
      const r = rank(text, term);
      if (r.rank < Math.max(min, phrase ? RANK.contains : RANK.letters)) continue;
      const capped = Math.min(r.rank, max);
      if (capped > top.rank || (capped === top.rank && r.close > top.close)) top = { rank: capped, close: r.close, field };
    }
  });
  return top;
}

const weaker = (a: Score, b: Score) => (b.rank < a.rank || (b.rank === a.rank && b.close < a.close) ? b : a);

/** Entries for `query`, best first. */
export function search(docs: readonly Doc[], query: string, limit = 60): Hit[] {
  const q = fold(query);
  if (!q) return [];
  const words = [...new Set(q.split(' '))];
  const several = words.length > 1;
  const hits: (Score & { doc: Doc; tier: number })[] = [];
  for (const doc of docs) {
    let score = best(doc, q, several);
    let tier = 0;
    if (score.rank === RANK.none && several) {
      const each = words.map((w) => best(doc, w, false));
      if (each.every((s) => s.rank > RANK.none)) {
        score = each.reduce(weaker);
        tier = 1;
      }
    }
    if (score.rank > RANK.none) hits.push({ ...score, doc, tier });
  }
  hits.sort(
    (a, b) =>
      a.tier - b.tier || b.rank - a.rank || b.close - a.close || a.field - b.field || a.doc.order - b.doc.order,
  );
  return hits.slice(0, limit).map((h) => ({ entry: h.doc.entry, page: h.doc.page }));
}
