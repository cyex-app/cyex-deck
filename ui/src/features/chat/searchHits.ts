// A10 Which words of a search to mark in its results.

/** The plain words in a query: operators (`from:x`, `regex:…`) are left out, quotes join words. */
export function searchTerms(query: string): string[] {
  const out: string[] = [];
  for (const m of query.matchAll(/"([^"]+)"|(\S+)/g)) {
    const quoted = m[1];
    if (quoted) out.push(quoted.toLowerCase());
    else if (!/^!?[a-z]+:/i.test(m[2]!)) out.push(m[2]!.toLowerCase());
  }
  return out.filter((t) => t.length > 0);
}

/** Ranges over every case-insensitive match of `terms` in the text under `root`, in reading order. */
export function findHits(root: HTMLElement, terms: string[]): Range[] {
  if (!terms.length) return [];
  const ranges: Range[] = [];
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  for (let n = walker.nextNode(); n; n = walker.nextNode()) {
    const parent = n.parentElement;
    if (!parent || parent.closest('.chan, .time, button')) continue;
    const text = n.textContent!.toLowerCase();
    for (const t of terms) {
      for (let i = text.indexOf(t); i >= 0; i = text.indexOf(t, i + t.length)) {
        const r = new Range();
        r.setStart(n, i);
        r.setEnd(n, i + t.length);
        ranges.push(r);
      }
    }
  }
  return ranges;
}
