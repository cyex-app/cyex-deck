// V110-F84 A refresh of an open conversation brings its newest page. It joins the lines
// already shown (older pages loaded by scrolling up stay) instead of replacing them.

/** The shown lines with the fresh ones merged in by id (fresh wins), oldest first, newest `max` kept. */
export function mergeLines<T extends { id: string; ts: number }>(shown: readonly T[], fresh: readonly T[], max: number): T[] {
  const byId = new Map<string, T>();
  for (const m of shown) byId.set(m.id, m);
  for (const m of fresh) byId.set(m.id, m);
  // Stable: lines with the same time keep the order they came in.
  return [...byId.values()].sort((a, b) => a.ts - b.ts).slice(-max);
}
