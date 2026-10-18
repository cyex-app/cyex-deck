// V106-25 Tab mute: shown at once, then Rust's answer. s3-9 Only the newest toggle may write the
// answer, and a failed one is undone only when no newer toggle of that tab came after it.

export interface MuteDeps {
  get: () => Set<string>;
  set: (ids: Set<string>) => void;
  send: (id: string, on: boolean) => Promise<string[]>;
  fail: (e: unknown) => void;
}

/** The set with `id` in or out. */
export function withId(set: Set<string>, id: string, on: boolean): Set<string> {
  const next = new Set(set);
  if (on) next.add(id);
  else next.delete(id);
  return next;
}

export function muter(d: MuteDeps): (id: string, on: boolean) => Promise<void> {
  let last = 0;
  const lastOf = new Map<string, number>();
  return async (id, on) => {
    const n = ++last;
    lastOf.set(id, n);
    d.set(withId(d.get(), id, on));
    try {
      const ids = await d.send(id, on);
      if (n === last) d.set(new Set(ids));
    } catch (e) {
      if (lastOf.get(id) === n) d.set(withId(d.get(), id, !on));
      d.fail(e);
    }
  };
}
