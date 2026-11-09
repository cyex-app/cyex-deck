// V110-F86 `@` completion: who can be mentioned, matched by display name or login.

import { rankNames } from '../emotes/ranking.ts';
import { complete } from './completion.ts';

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

/** The mention names matching `typed` by name or display name, best first. */
export function matchPeople(typed: string, people: readonly Person[], limit = 8): string[] {
  const owner = new Map<string, string>();
  for (const p of people) {
    if (!owner.has(p.name)) owner.set(p.name, p.name);
    if (!owner.has(p.display)) owner.set(p.display, p.name);
  }
  const out: string[] = [];
  for (const c of complete(typed, owner.keys(), owner.size)) {
    const n = owner.get(c) as string;
    if (!out.includes(n)) out.push(n);
    if (out.length >= limit) break;
  }
  return out;
}

/** Tab's matches for `word`: after `@` only people (with the `@`), else emotes and people. */
export function tabNames(word: string, emotes: readonly string[], people: readonly Person[], limit = 30): string[] {
  const bare = word.replace(/^@/, '');
  if (!bare) return [];
  if (word.startsWith('@')) return matchPeople(bare, people, limit).map((n) => `@${n}`);
  return rankNames(bare, [...emotes, ...people.map((p) => p.name)], limit);
}
