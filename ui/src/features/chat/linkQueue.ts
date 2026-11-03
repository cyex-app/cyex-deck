// V108-C25 Link cards asked for on a hover (1.52). Each link is asked once at a time (a second
// hover joins the ask already out). Rust keeps the answers (30 minutes, misses 2), so this only
// shares the ask in flight. F46b The ahead-of-hover prefetch had no caller and went.

export interface LinkQueue<T> {
  /** For a hover: joins the ask out for `url`, or asks now. */
  get(url: string): Promise<T>;
  /** Asks running, for tests. */
  stats(): { running: number };
}

export function linkQueue<T>(ask: (url: string) => Promise<T>): LinkQueue<T> {
  const out = new Map<string, Promise<T>>();

  return {
    get(url) {
      const known = out.get(url);
      if (known) return known;
      const p = Promise.resolve()
        .then(() => ask(url))
        .finally(() => out.delete(url));
      out.set(url, p);
      return p;
    },
    stats: () => ({ running: out.size }),
  };
}
