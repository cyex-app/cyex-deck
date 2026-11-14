// V108-C11 Names inside a notice's text (the gifter, a lone recipient) that open their usercard.

import type { NoticeUser } from '../../ipc/bindings/NoticeUser';

export type NoticePart = { text: string; login?: string; color?: NoticeUser['color'] };

const WORD = /[\p{L}\p{N}_]/u;

/** Where `name` first stands alone in `hay` outside the claimed ranges, or -1. */
function scan(text: string, hay: string, n: string, taken: readonly [number, number][]): number {
  for (let at = hay.indexOf(n); at >= 0; at = hay.indexOf(n, at + 1)) {
    const end = at + n.length;
    if (taken.some(([a, b]) => at < b && end > a)) continue;
    if ((at > 0 && WORD.test(text[at - 1]!)) || (end < text.length && WORD.test(text[end]!))) continue;
    return at;
  }
  return -1;
}

/** Where `name` stands alone in `text` outside the claimed ranges, or -1: as spelled first
 *  (V111-16 so a mod named "A" doesn't claim the word "a"), else in any case. */
function find(text: string, lower: string, name: string, taken: readonly [number, number][]): number {
  if (!name) return -1;
  const exact = scan(text, text, name, taken);
  return exact >= 0 ? exact : scan(text, lower, name.toLowerCase(), taken);
}

/** The text cut into plain runs and user names. A name not found stays plain. */
export function splitNames(text: string, links: readonly NoticeUser[]): NoticePart[] {
  const lower = text.toLowerCase();
  // Casing that changes the length would shift every index: match as written.
  if (!links.length || lower.length !== text.length) return [{ text }];
  const taken: [number, number][] = [];
  const hits: { at: number; end: number; login: string; color?: NoticeUser['color'] }[] = [];
  // Longer names first, so "Bob" doesn't claim the start of "Bob_2".
  for (const l of [...links].sort((a, b) => b.name.length - a.name.length)) {
    const at = find(text, lower, l.name, taken);
    if (at < 0) continue;
    const end = at + l.name.length;
    taken.push([at, end]);
    hits.push({ at, end, login: l.login, ...(l.color && { color: l.color }) });
  }
  hits.sort((a, b) => a.at - b.at);
  const parts: NoticePart[] = [];
  let pos = 0;
  for (const h of hits) {
    if (h.at > pos) parts.push({ text: text.slice(pos, h.at) });
    parts.push({ text: text.slice(h.at, h.end), login: h.login, ...(h.color && { color: h.color }) });
    pos = h.end;
  }
  if (pos < text.length) parts.push({ text: text.slice(pos) });
  return parts;
}

/**
 * V111-01 Where an emote-change line's picture goes: the text cut right after the emote's name
 * (the one after "emote ", "to " or "now ", else its last place). Null when the name isn't there.
 */
export function splitEmote(text: string, name: string): [string, string] | null {
  if (!name) return null;
  let at = -1;
  for (const lead of ['emote ', ' to ', ' now ']) {
    const i = text.indexOf(lead + name);
    if (i < 0) continue;
    const end = i + lead.length + name.length;
    // The name stands alone: a longer name ("notLX") doesn't count.
    if (end < text.length && WORD.test(text[end]!)) continue;
    at = end;
    // "renamed the 7TV emote a to b": the last of these is the new name.
  }
  if (at < 0) {
    const i = text.lastIndexOf(name);
    if (i < 0) return null;
    at = i + name.length;
  }
  return [text.slice(0, at), text.slice(at)];
}
