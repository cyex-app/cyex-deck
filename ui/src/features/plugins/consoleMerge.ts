// V110-F84 A plugin console opens by subscribing to the live log first, then reading the
// snapshot. Lines that came in while the snapshot was on its way may be in both: they are
// counted once, and the rest follow the snapshot in time order.

export interface LogEntry {
  ts: number;
  kind: string;
  level?: string;
  text: string;
}

const same = (a: LogEntry, b: LogEntry) => a.ts === b.ts && a.kind === b.kind && a.level === b.level && a.text === b.text;

/** The snapshot plus the `live` lines it doesn't hold yet (a repeat in `live` matches one snapshot line each), newest `max` kept. */
export function mergeLog<T extends LogEntry>(snapshot: readonly T[], live: readonly T[], max: number): T[] {
  const left = [...snapshot];
  const extra: T[] = [];
  for (const e of live) {
    const i = left.findIndex((s) => same(s, e));
    if (i >= 0) left.splice(i, 1);
    else extra.push(e);
  }
  // Stable: equal times keep their order.
  return [...snapshot, ...extra].sort((a, b) => a.ts - b.ts).slice(-max);
}
