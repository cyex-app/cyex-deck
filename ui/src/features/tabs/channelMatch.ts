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

/** Paths on twitch.tv that aren't a channel. */
const NOT_CHANNELS = new Set(['videos', 'directory', 'downloads', 'settings', 'subscriptions', 'wallet', 'inventory', 'p', 'search', 'login', 'signup']);

/** The login in a pasted twitch.tv link (`https://www.twitch.tv/rvbdev?sr=a`, `twitch.tv/RvbDev/about`,
 *  `twitch.tv/popout/rvbdev/chat`), lower case; null when the text isn't such a link. */
export function loginFromLink(typed: string): string | null {
  const m = /^(?:https?:\/\/)?(?:www\.|m\.|go\.)?twitch\.tv\/(?:popout\/|moderator\/)?([A-Za-z0-9_]{1,25})(?:[/?#].*)?$/i.exec(typed.trim());
  const login = m?.[1].toLowerCase();
  return login && !NOT_CHANNELS.has(login) ? login : null;
}

/** V108-F12 (S3-01) What Enter does in the channel field. `wait`: the list still belongs to an older
 *  text (or is being fetched), so look up what's typed now first. A channel: add it. null: nothing to add. */
export function enterTarget<T extends { login: string }>(
  typed: string,
  asked: string,
  status: 'idle' | 'looking' | 'done' | 'error',
  results: T[],
  picked: T | null,
): T | 'wait' | null {
  const t = typed.trim();
  if (!t) return null;
  if (picked && (status !== 'looking' || asked === t)) return picked;
  if (status !== 'done' || asked !== t) return status === 'error' && asked === t ? null : 'wait';
  return results[0] ?? null;
}

/** V110-F28 Whether typed text could name a channel: a login (4 to 25 of a-z, 0-9 and _, after
 *  an @ or # and case are dropped) or a twitch.tv link. The switcher offers "Open …" only then. */
export function couldBeChannel(typed: string): boolean {
  const t = typed.trim();
  if (loginFromLink(t)) return true;
  return /^[a-z0-9_]{4,25}$/.test(t.replace(/^[@#]+/, '').toLowerCase());
}
