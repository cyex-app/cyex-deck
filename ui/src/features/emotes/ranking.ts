// V107-R7 One emote order for the emote menu's search (1.40) and Tab completion (C6):
// the exact name first, then how well the name matches, then the emotes you send most
// (C12), then the order given (menu order). Match comes before use, so a used
// contains-match never jumps the prefix matches.

import type { EmoteKey } from '../../ipc/bindings/EmoteKey';
import { RANK, rank } from '../chat/completion.ts';
import { emoteKey } from './emotes.ts';

export interface Candidate<T> {
  item: T;
  name: string;
  /** Place in the most-used list, lower is more; `Infinity` (or none) for never used. */
  used?: number;
  /** Locked emotes (E5) come after every usable one. */
  locked?: boolean;
}

/** C12 Each emote's place in the most-used list, by `provider:id`. */
export function usageRank(usage: readonly { emote: EmoteKey }[]): Map<string, number> {
  const out = new Map<string, number>();
  usage.forEach((u, i) => {
    const k = emoteKey(u.emote.source, u.emote.id);
    if (!out.has(k)) out.set(k, i);
  });
  return out;
}

/** The candidates matching `typed`, best first. `least`: the weakest match kept (the menu
 *  keeps names containing what's typed, Tab also takes acronyms and letters in order). */
export function rankEmotes<T>(typed: string, candidates: Iterable<Candidate<T>>, least: number = RANK.letters): T[] {
  const q = typed.trim();
  if (!q) return [];
  const hits: { item: T; locked: boolean; rank: number; close: number; used: number; i: number }[] = [];
  let i = 0;
  for (const c of candidates) {
    const r = rank(c.name, q);
    if (r.rank < Math.max(least, RANK.letters)) continue;
    hits.push({ item: c.item, locked: !!c.locked, ...r, used: c.used ?? Infinity, i: i++ });
  }
  // `rank` puts the exact name (any case) above every prefix match.
  hits.sort(
    (a, b) =>
      Number(a.locked) - Number(b.locked) ||
      b.rank - a.rank ||
      (a.used === b.used ? 0 : a.used < b.used ? -1 : 1) ||
      b.close - a.close ||
      a.i - b.i,
  );
  return hits.map((h) => h.item);
}

/** C6 Tab and `:` completion: names in the same order, each once, what's typed exactly
 *  left out. `used` as in `Candidate`. */
export function rankNames(
  typed: string,
  names: Iterable<string>,
  limit = 8,
  used: (name: string) => number = () => Infinity,
): string[] {
  const seen = new Set<string>();
  const all: Candidate<string>[] = [];
  for (const n of names) {
    if (seen.has(n) || n === typed) continue;
    seen.add(n);
    all.push({ item: n, name: n, used: used(n) });
  }
  return rankEmotes(typed, all).slice(0, limit);
}
