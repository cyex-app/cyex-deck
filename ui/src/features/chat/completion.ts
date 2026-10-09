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

/** Candidates for `prefix`: starts-with matches first, then contains, case-insensitive. */
export function complete(prefix: string, candidates: Iterable<string>, limit = 8): string[] {
  const p = prefix.toLowerCase();
  if (!p) return [];
  const starts: string[] = [];
  const contains: string[] = [];
  const seen = new Set<string>();
  for (const c of candidates) {
    if (seen.has(c)) continue;
    seen.add(c);
    const l = c.toLowerCase();
    if (l === p) continue;
    if (l.startsWith(p)) starts.push(c);
    else if (l.includes(p)) contains.push(c);
  }
  const byLength = (a: string, b: string) => a.length - b.length || a.localeCompare(b);
  return [...starts.sort(byLength), ...contains.sort(byLength)].slice(0, limit);
}

/** Replaces the word with `value` plus a space. Returns the new text and caret. */
export function insert(text: string, word: Word, value: string): { text: string; caret: number } {
  const before = text.slice(0, word.start);
  const after = text.slice(word.end).replace(/^\s?/, ' ');
  return { text: before + value + after, caret: before.length + value.length + 1 };
}
