// C6 Tab, `@` and `:` completion: chatters and emotes matching what's typed.

export interface Word {
  start: number;
  end: number;
  text: string;
}

/** The word around the caret, up to the caret. */
export function wordAt(text: string, caret: number): Word {
  let start = caret;
  while (start > 0 && !/\s/.test(text[start - 1])) start--;
  return { start, end: caret, text: text.slice(start, caret) };
}

/** V101-16 How well a name matches what's typed, best first. Match-sorter's order (Kent C.
 * Dodds): exact, starts with, a word starts with, contains, acronym, letters in order.
 * Emote names have no spaces, so words split where the case changes, at digits and at
 * `_` or `-` ("peepoHappy" is "peepo" "Happy", acronym "ph"). Settings search (V103-13)
 * ranks with it too, so spaces split words as well. */
export const RANK = { caseEqual: 7, equal: 6, starts: 5, word: 4, contains: 3, acronym: 2, letters: 1, none: 0 } as const;

const kind = (ch: string) => (/\d/.test(ch) ? 'digit' : /[a-z]/.test(ch) ? 'lower' : /[A-Z]/.test(ch) ? 'upper' : 'other');
const gap = (ch: string) => ch === '_' || ch === '-' || /\s/.test(ch);

/** Where each word of `name` starts. */
export function wordStarts(name: string): number[] {
  const out: number[] = [];
  for (let i = 0; i < name.length; i++) {
    const c = name[i];
    if (gap(c)) continue;
    if (i === 0) {
      out.push(i);
      continue;
    }
    const prev = name[i - 1];
    const k = kind(c);
    const pk = kind(prev);
    if (gap(prev) || (k === 'upper' && pk === 'lower') || (k === 'digit') !== (pk === 'digit')) out.push(i);
  }
  return out;
}

/** The rank of `name` for `typed` (both as given), and for letters in order how close
 * together they sit, 0 to 1. */
export function rank(name: string, typed: string): { rank: number; close: number } {
  const raw = rankRaw(name, typed);
  const fn = fold(name);
  const ft = fold(typed);
  if (fn === name && ft === typed) return raw;
  // V107-MI Accents don't block a match ("cafe" finds "café"); an exact accented match
  // keeps the higher rank, the folded one tops out at `equal`.
  const f = rankRaw(fn, ft);
  const folded = f.rank === RANK.caseEqual ? { rank: RANK.equal, close: 1 } : f;
  return folded.rank > raw.rank || (folded.rank === raw.rank && folded.close > raw.close) ? folded : raw;
}

/** V107-MI Case kept, accents (combining marks) dropped: "Café" is "Cafe". */
export function fold(s: string): string {
  return /[^\x00-\x7f]/.test(s) ? s.normalize('NFD').replace(/\p{M}+/gu, '') : s;
}

function rankRaw(name: string, typed: string): { rank: number; close: number } {
  const none = { rank: RANK.none, close: 0 };
  if (!typed) return none;
  if (name === typed) return { rank: RANK.caseEqual, close: 1 };
  const n = name.toLowerCase();
  const t = typed.toLowerCase();
  if (n === t) return { rank: RANK.equal, close: 1 };
  if (n.startsWith(t)) return { rank: RANK.starts, close: 1 };
  const starts = wordStarts(name);
  if (starts.some((i) => i > 0 && n.startsWith(t, i))) return { rank: RANK.word, close: 1 };
  if (n.includes(t)) return { rank: RANK.contains, close: 1 };
  if (t.length === 1) return none;
  if (starts.map((i) => n[i]).join('').includes(t)) return { rank: RANK.acronym, close: 1 };
  let first = -1;
  let at = -1;
  for (const ch of t) {
    at = n.indexOf(ch, at + 1);
    if (at < 0) return none;
    if (first < 0) first = at;
  }
  return { rank: RANK.letters, close: 1 / Math.max(at - first, 1) };
}

/** The best `rank` of several names for `typed`: the highest rank, then the highest close. */
export function rankBest(names: readonly string[], typed: string): { rank: number; close: number } {
  let best = rank('', '');
  for (const n of names) {
    const r = rank(n, typed);
    if (r.rank > best.rank || (r.rank === best.rank && r.close > best.close)) best = r;
  }
  return best;
}

/** Candidates for `prefix`, best match first (`rank`), then the shortest. What's typed
 * exactly is left out. Emotes rank in emotes/ranking.ts instead (V107-R7). */
export function complete(
  prefix: string,
  candidates: Iterable<string>,
  limit = 8,
): string[] {
  if (!prefix) return [];
  const hits: { name: string; rank: number; close: number }[] = [];
  const seen = new Set<string>();
  for (const c of candidates) {
    if (seen.has(c) || c === prefix) continue;
    seen.add(c);
    const r = rank(c, prefix);
    if (r.rank > RANK.none) hits.push({ name: c, ...r });
  }
  hits.sort(
    (a, b) =>
      b.rank - a.rank ||
      b.close - a.close ||
      a.name.length - b.name.length ||
      a.name.localeCompare(b.name),
  );
  return hits.slice(0, limit).map((h) => h.name);
}

/** Replaces the word with `value` plus a space. Returns the new text and caret. */
export function insert(text: string, word: Word, value: string): { text: string; caret: number } {
  const before = text.slice(0, word.start);
  const after = text.slice(word.end).replace(/^\s?/, ' ');
  return { text: before + value + after, caret: before.length + value.length + 1 };
}

/** V108-MC "Mention users with a comma": a Tab cycle that puts `name,` (insert adds the space).
 * Only for the first word of the message; stage `name` is a typed partial, whose first Tab
 * still shows the plain name. */
export interface CommaPlan {
  /** User names only, as shown (with `@` if it was typed). */
  items: string[];
  index: number;
  stage: 'name' | 'comma';
}

/** V108-MC The word starts the message and isn't a command. */
export function startsMessage(text: string, word: Word): boolean {
  return text.slice(0, word.start).trim() === '' && !word.text.startsWith('/');
}

/** V108-MC The comma cycle for these matches, or null when the best one isn't a user (emotes,
 * plugin values: unchanged). An `@word` or an already complete name gets its comma at once. */
export function commaStart(items: readonly string[], typed: string, isUser: (item: string) => boolean): CommaPlan | null {
  if (!items.length || !isUser(items[0])) return null;
  const same = items[0].toLowerCase() === typed.toLowerCase();
  return { items: items.filter(isUser), index: 0, stage: typed.startsWith('@') || same ? 'comma' : 'name' };
}

/** V108-MC What goes in the box for this step: the name, or the name with its comma. */
export function commaValue(plan: CommaPlan): string {
  const name = plan.items[plan.index];
  return plan.stage === 'comma' ? `${name},` : name;
}

/** V108-MC The next Tab: the plain name gets its comma first, then the other users cycle. */
export function commaStep(plan: CommaPlan, back: boolean): CommaPlan {
  if (plan.stage === 'name' && !back) return { ...plan, stage: 'comma' };
  const n = plan.items.length;
  return { ...plan, stage: 'comma', index: (plan.index + (back ? n - 1 : 1)) % n };
}
