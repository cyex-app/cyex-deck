// 1.14 Ranks the channel field's suggestions with the shared match sorter (`rank`, the one the
// emote completion and the settings search use). The exact lookup (name, id or link) always
// stays in, offline or not. Ties: live first, then the order they came in.

import { rankBest, RANK } from '../chat/completion.ts';

/** What a suggestion needs; `ChannelInfo` fits. */
export interface Candidate {
  id: string;
  login: string;
  name: string;
  stream: unknown;
}

/** Above any `rank`: what the exact lookup found matched by definition (an id or link too). */
const EXACT = RANK.caseEqual + 1;

/** `exact` from the lookup, `search` in Twitch's order; deduped by id, at most `max`. */
export function matchChannels<T extends Candidate>(exact: T[], search: T[], typed: string, max: number): T[] {
  const input = typed.trim();
  const seen = new Set<string>();
  const scored: { c: T; score: number; close: number; i: number }[] = [];
  for (const [i, c] of [...exact, ...search].entries()) {
    if (seen.has(c.id)) continue;
    seen.add(c.id);
    if (i < exact.length) {
      scored.push({ c, score: EXACT, close: 1, i });
      continue;
    }
    const best = rankBest([c.login, c.name], input);
    scored.push({ c, score: best.rank, close: best.close, i });
  }
  return scored
    .sort((a, b) => b.score - a.score || b.close - a.close || Number(!!b.c.stream) - Number(!!a.c.stream) || a.i - b.i)
    .slice(0, max)
    .map((x) => x.c);
}

/** U1 The switcher lists live channels from search, plus the one whose name is exactly what's
 * typed even while offline. */
export function liveOrExact<T extends Candidate>(items: T[], typed: string): T[] {
  const q = typed.trim().toLowerCase();
  return items.filter((c) => c.stream || c.login === q || c.name.toLowerCase() === q);
}
