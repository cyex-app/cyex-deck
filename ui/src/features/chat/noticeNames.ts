// V108-C11 Names inside a notice's text (the gifter, a lone recipient) that open their usercard.

import type { NoticeUser } from '../../ipc/bindings/NoticeUser';

export type NoticePart = { text: string; login?: string };

const WORD = /[\p{L}\p{N}_]/u;

/** Where `name` first stands alone in `text` (any case) outside the claimed ranges, or -1. */
function find(text: string, lower: string, name: string, taken: readonly [number, number][]): number {
  const n = name.toLowerCase();
  if (!n) return -1;
  for (let at = lower.indexOf(n); at >= 0; at = lower.indexOf(n, at + 1)) {
    const end = at + n.length;
    if (taken.some(([a, b]) => at < b && end > a)) continue;
    if ((at > 0 && WORD.test(text[at - 1]!)) || (end < text.length && WORD.test(text[end]!))) continue;
    return at;
  }
  return -1;
}

/** The text cut into plain runs and user names. A name not found stays plain. */
export function splitNames(text: string, links: readonly NoticeUser[]): NoticePart[] {
  const lower = text.toLowerCase();
  // Casing that changes the length would shift every index: match as written.
  if (!links.length || lower.length !== text.length) return [{ text }];
  const taken: [number, number][] = [];
  const hits: { at: number; end: number; login: string }[] = [];
  // Longer names first, so "Bob" doesn't claim the start of "Bob_2".
  for (const l of [...links].sort((a, b) => b.name.length - a.name.length)) {
    const at = find(text, lower, l.name, taken);
    if (at < 0) continue;
    const end = at + l.name.length;
    taken.push([at, end]);
    hits.push({ at, end, login: l.login });
  }
  hits.sort((a, b) => a.at - b.at);
  const parts: NoticePart[] = [];
  let pos = 0;
  for (const h of hits) {
    if (h.at > pos) parts.push({ text: text.slice(pos, h.at) });
    parts.push({ text: text.slice(h.at, h.end), login: h.login });
    pos = h.end;
  }
  if (pos < text.length) parts.push({ text: text.slice(pos) });
  return parts;
}
