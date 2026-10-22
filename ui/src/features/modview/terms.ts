// V107-R31 The blocked terms panel's list: search, sort, the add check (as Twitch takes
// them: 2 to 500 characters), and which mod actions mean the list changed.

import type { BlockedTerm } from '../../ipc/bindings/BlockedTerm';
import type { ModLine } from '../../ipc/bindings/ModLine';

export type TermSort = 'new' | 'old' | 'az';

export const SORTS: { value: TermSort; label: string }[] = [
  { value: 'new', label: 'Newest' },
  { value: 'old', label: 'Oldest' },
  { value: 'az', label: 'A–Z' },
];

export function shownTerms(list: BlockedTerm[], query: string, sort: TermSort): BlockedTerm[] {
  const q = query.trim().toLowerCase();
  const hit = q ? list.filter((t) => t.text.toLowerCase().includes(q)) : [...list];
  if (sort === 'az') return hit.sort((a, b) => a.text.localeCompare(b.text, undefined, { sensitivity: 'base' }));
  const dir = sort === 'new' ? -1 : 1;
  return hit.sort((a, b) => dir * a.createdAt.localeCompare(b.createdAt) || a.text.localeCompare(b.text));
}

/** Why `text` can't be added, or `null`. */
export function termProblem(text: string, list: BlockedTerm[]): string | null {
  const t = text.trim();
  const n = [...t].length;
  if (n < 2) return 'A term needs at least 2 characters.';
  if (n > 500) return 'A term can have up to 500 characters.';
  if (list.some((b) => b.text.toLowerCase() === t.toLowerCase())) return 'That term is blocked already.';
  return null;
}

/** Rust's words for a blocked term added or removed (`moderation::actions`, V107-R44). */
const TERM_WORDS = ['added the blocked term', 'removed the blocked term'];

/** A live mod action that changed the blocked terms (`channel.moderate`). V107-N3a It
 *  matched the old words ("blocked"), so the list never followed other mods' changes. */
export const changesTerms = (l: ModLine): boolean => !l.past && TERM_WORDS.includes(l.action);
