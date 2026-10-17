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
