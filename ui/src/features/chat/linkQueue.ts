// V108-C25 Link cards asked for ahead of a hover (1.52). Each link is asked once at a time
// (a hover joins the ask already out), at most `limit` at once, newest lines first; a long
// backlog drops its oldest asks, which have scrolled away. Rust keeps the answers (30 minutes,
// misses 2), so this only remembers what it already asked for lately.

export interface LinkQueue<T> {
  /** For a hover: joins the ask out for `url`, or asks now, ahead of the queue. */
  get(url: string): Promise<T>;
  /** For a line that came in: asks later, when there's room. */
  prefetch(url: string): void;
  /** Asks running and waiting, for tests. */
  stats(): { running: number; waiting: number };
}

export interface QueueOptions {
  /** At most this many at once. */
  limit?: number;
  /** Waiting asks kept; older ones go. */
  backlog?: number;
  /** Links asked lately, not asked again by a prefetch. */
  remember?: number;
}

export function linkQueue<T>(ask: (url: string) => Promise<T>, o: QueueOptions = {}): LinkQueue<T> {
  const limit = o.limit ?? 4;
  const backlog = o.backlog ?? 24;
  const remember = o.remember ?? 500;
  const out = new Map<string, Promise<T>>();
  /** Oldest first; the newest is taken first. */
  let waiting: string[] = [];
  /** Insertion order is age: the first key is the oldest. */
  const asked = new Set<string>();
  let running = 0;

  function note(url: string) {
    asked.delete(url);
    asked.add(url);
    if (asked.size > remember) asked.delete(asked.values().next().value!);
  }

  function run(url: string): Promise<T> {
    const known = out.get(url);
    if (known) return known;
    note(url);
    running++;
    const p = Promise.resolve()
      .then(() => ask(url))
      .finally(() => {
        out.delete(url);
        running--;
        pump();
      });
    out.set(url, p);
    return p;
  }

  function pump() {
    while (running < limit && waiting.length) run(waiting.pop()!).catch(() => {});
  }

  return {
    get(url) {
      waiting = waiting.filter((w) => w !== url);
      return run(url);
    },
    prefetch(url) {
      if (out.has(url) || asked.has(url) || waiting.includes(url)) return;
      waiting.push(url);
      if (waiting.length > backlog) waiting.shift();
      pump();
    },
    stats: () => ({ running, waiting: waiting.length }),
  };
}
