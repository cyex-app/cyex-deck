// V110-F86 `@` completion: who can be mentioned, matched by display name or login.

import { rankNames } from '../emotes/ranking.ts';
import { RANK, rankBest } from './completion.ts';

export interface Person {
  /** What goes after the `@`. */
  name: string;
  /** What chat shows; may be localized. */
  display: string;
}

/** Twitch pings by login: a display name that differs from it by more than case (a localized
 *  one) is mentioned by login. */
export function mentionName(display: string, login: string): string {
  return !login || display.toLowerCase() === login.toLowerCase() ? display : login;
}

/** V111-41 Each login's place in the most-mentioned list (lower is more), from `emotes.mentions`. */
export function mentionRank(mentions: readonly { login: string }[]): Map<string, number> {
  const out = new Map<string, number>();
  mentions.forEach((m, i) => {
    if (!out.has(m.login)) out.set(m.login, i);
  });
  return out;
}

/** The mention names matching `typed` by name or display name, best first. V111-41 Within the
 *  same kind of match, people you mention most come first (`used`, by lowercase login). */
export function matchPeople(typed: string, people: readonly Person[], limit = 8, used: ReadonlyMap<string, number> = new Map()): string[] {
  if (!typed) return [];
  const hits: { name: string; rank: number; close: number; used: number }[] = [];
  const seen = new Set<string>();
  for (const p of people) {
    if (seen.has(p.name) || p.name === typed) continue;
    seen.add(p.name);
    const r = rankBest(p.display === p.name ? [p.name] : [p.name, p.display], typed);
    if (r.rank > RANK.none) hits.push({ name: p.name, ...r, used: used.get(p.name.toLowerCase()) ?? Infinity });
  }
  hits.sort(
    (a, b) =>
      b.rank - a.rank ||
      (a.used === b.used ? 0 : a.used < b.used ? -1 : 1) ||
      b.close - a.close ||
      a.name.length - b.name.length ||
      a.name.localeCompare(b.name),
  );
  return hits.slice(0, limit).map((h) => h.name);
}

/** Tab's matches for `word`: after `@` only people (with the `@`), else emotes and people. */
export function tabNames(
  word: string,
  emotes: readonly string[],
  people: readonly Person[],
  limit = 30,
  used: ReadonlyMap<string, number> = new Map(),
): string[] {
  const bare = word.replace(/^@/, '');
  if (!bare) return [];
  if (word.startsWith('@')) return matchPeople(bare, people, limit, used).map((n) => `@${n}`);
  return rankNames(bare, [...emotes, ...people.map((p) => p.name)], limit);
}
