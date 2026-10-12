// A settings edit shows at once, before Rust answers (V104-04). Two edits faster than the
// answer (a word typed into a list, then a box ticked in the same list) used to both start
// from the old list, so the second one undid the first.

/** JSON merge patch as Rust applies it: objects merge, arrays and values replace. `null`
 * resets to the default, which only Rust knows, so it keeps the value until Rust answers. */
export function mergePatch<T>(target: T, patch: unknown): T {
  if (patch === null || typeof patch !== 'object' || Array.isArray(patch)) return patch as T;
  const base = target !== null && typeof target === 'object' && !Array.isArray(target) ? target : {};
  const out: Record<string, unknown> = { ...(base as Record<string, unknown>) };
  for (const [k, v] of Object.entries(patch as Record<string, unknown>)) {
    if (v === null) continue;
    out[k] = mergePatch(out[k], v);
  }
  return out as T;
}

/** Counts edits per key in flight: only the answer to the newest one is kept, since Rust
 * applies them in order and that answer holds them all. */
export class InFlight<K extends string> {
  private last = new Map<K, number>();

  /** A new edit to `key`; pass the number back to `settle`. */
  start(key: K): number {
    const n = (this.last.get(key) ?? 0) + 1;
    this.last.set(key, n);
    return n;
  }

  /** True when `n` is the newest edit to `key`: its answer is the one to keep. */
  settle(key: K, n: number): boolean {
    if (this.last.get(key) !== n) return false;
    this.last.delete(key);
    return true;
  }

  /** Edits to `key` still waiting for an answer. */
  busy(key: K): boolean {
    return this.last.has(key);
  }
}
