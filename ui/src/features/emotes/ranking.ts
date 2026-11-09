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

/** V110-U71 The order of Tab and `:` completion. The closest match leads: the name that equals
 *  what's typed in any case, else the shortest name starting with it (ties A-Z). The rest
 *  follow by how they match (prefix, word start, contains, acronym), each A-Z in any case.
 *  `tie` breaks a dead heat (same name, same match): lower first, e.g. the provider's place
 *  in the priority list. `least`: the weakest match kept. */
export function orderHits<T>(
  typed: string,
  candidates: Iterable<{ item: T; name: string; tie?: number }>,
  least: number = RANK.acronym,
): T[] {
  const q = typed.trim();
  if (!q) return [];
  const hits: { item: T; name: string; low: string; rank: number; close: number; tie: number }[] = [];
  for (const c of candidates) {
    const r = rank(c.name, q);
    if (r.rank < Math.max(least, RANK.letters)) continue;
    hits.push({ item: c.item, name: c.name, low: c.name.toLowerCase(), ...r, tie: c.tie ?? 0 });
  }
  const az = (a: { low: string; name: string }, b: { low: string; name: string }) =>
    a.low < b.low ? -1 : a.low > b.low ? 1 : a.name < b.name ? -1 : a.name > b.name ? 1 : 0;
  let lead: (typeof hits)[number] | undefined;
  for (const h of hits) {
    if (h.rank >= RANK.equal) {
      if (!lead || lead.rank < RANK.equal || h.rank > lead.rank || (h.rank === lead.rank && (h.tie - lead.tie || az(h, lead)) < 0)) lead = h;
    } else if (h.rank === RANK.starts && (!lead || lead.rank < RANK.equal)) {
      if (!lead || h.name.length < lead.name.length || (h.name.length === lead.name.length && (az(h, lead) || h.tie - lead.tie) < 0)) lead = h;
    }
  }
  const rest = hits
    .filter((h) => h !== lead)
    .sort((a, b) => b.rank - a.rank || (a.rank === RANK.letters ? b.close - a.close : 0) || az(a, b) || a.tie - b.tie);
  return (lead ? [lead, ...rest] : rest).map((h) => h.item);
}

/** C6 Tab completion: names in `orderHits` order, each once, what's typed exactly left
 *  out. V110-F09 Letters merely in order don't count ("jul" isn't jinxlul): with no prefix,
 *  word, contains or acronym match, Tab does nothing. */
export function rankNames(typed: string, names: Iterable<string>, limit = 8): string[] {
  const seen = new Set<string>();
  const all: { item: string; name: string }[] = [];
  for (const n of names) {
    if (seen.has(n) || n === typed) continue;
    seen.add(n);
    all.push({ item: n, name: n });
  }
  return orderHits(typed, all, RANK.acronym).slice(0, limit);
}
