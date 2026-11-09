/** V106-21 Keys for a keyed `each` over outside data: a repeat gets `~2`, `~3`, … so
 *  Svelte never meets the same key twice (it throws, even in production). */
export function uniqueKeys<T>(list: readonly T[], key: (t: T) => string): string[] {
  const seen = new Map<string, number>();
  const used = new Set<string>();
  return list.map((t) => {
    const k = key(t);
    let n = (seen.get(k) ?? 0) + 1;
    let out = n === 1 ? k : `${k}~${n}`;
    while (used.has(out)) out = `${k}~${++n}`;
    seen.set(k, n);
    used.add(out);
    return out;
  });
}

/** V110-F81 An IME is turning keystrokes into text (Japanese, Chinese, Korean): its Enter confirms the
 *  candidate and must not send, search or save. Chromium reports keyCode 229 for it on keydown. */
export function composing(e: { isComposing?: boolean; keyCode?: number }): boolean {
  return !!e.isComposing || e.keyCode === 229;
}

/** V110-F81 A plain Enter press, i.e. not the one that confirms IME text. */
export function enterKey(e: { key: string; isComposing?: boolean; keyCode?: number }): boolean {
  return e.key === 'Enter' && !composing(e);
}
