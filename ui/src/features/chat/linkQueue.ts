// V108-C25 Link cards asked for on a hover (1.52). Each link is asked once at a time (a second
// hover joins the ask already out). Rust keeps the answers (30 minutes, misses 2), so this only
// shares the ask in flight. V110-C66 An optional limit on asks running at once. F46b The ahead-of-hover prefetch had no caller and went.

export interface LinkQueue<T> {
  /** For a hover: joins the ask out for `url`, or asks now. */
  get(url: string): Promise<T>;
  /** Asks running, for tests. */
  stats(): { running: number };
}

/** V110-C66 `limit`: at most this many asks run at once, the rest wait in order (the cards and
 *  asks ahead of a hover, so a fast chat can't start dozens of lookups). */
export function linkQueue<T>(ask: (url: string) => Promise<T>, limit = Infinity): LinkQueue<T> {
  const out = new Map<string, Promise<T>>();
  const waiting: (() => void)[] = [];
  let running = 0;

  const slot = (): Promise<void> => {
    if (running < limit) {
      running++;
      return Promise.resolve();
    }
    return new Promise((go) => waiting.push(() => { running++; go(); }));
  };
  const free = () => {
    running--;
    waiting.shift()?.();
  };

  return {
    get(url) {
      const known = out.get(url);
      if (known) return known;
      const p = slot()
        .then(() => ask(url).finally(free))
        .finally(() => out.delete(url));
      out.set(url, p);
      return p;
    },
    stats: () => ({ running: out.size }),
  };
}
