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
  /** V107-A8 The last change event skipped while an edit was in flight. */
  private missed = new Map<K, unknown>();

  /** A new edit to `key`; pass the number back to `settle`. */
  start(key: K): number {
    const n = (this.last.get(key) ?? 0) + 1;
    this.last.set(key, n);
    return n;
  }

  /** V107-A8 A change event for `key` arrived while busy and was skipped. */
  miss(key: K, value: unknown): void {
    if (this.last.has(key)) this.missed.set(key, value);
  }

  /** `null` unless `n` is the newest edit to `key` (its answer is the one to keep). Then
   * `missed` holds the last change event skipped meanwhile, if any. */
  settle(key: K, n: number): Settled | null {
    if (this.last.get(key) !== n) return null;
    this.last.delete(key);
    const settled: Settled = this.missed.has(key) ? { missed: { value: this.missed.get(key) } } : {};
    this.missed.delete(key);
    return settled;
  }

  /** Edits to `key` still waiting for an answer. */
  busy(key: K): boolean {
    return this.last.has(key);
  }
}

export interface Settled {
  missed?: { value: unknown };
}

/** V107-A8 Same JSON, whatever the key order (Rust's events and answers may differ in it). */
export function sameJson(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (a === null || b === null || typeof a !== 'object' || typeof b !== 'object') return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a)) {
    const bb = b as unknown[];
    return a.length === bb.length && a.every((v, i) => sameJson(v, bb[i]));
  }
  const ao = a as Record<string, unknown>;
  const bo = b as Record<string, unknown>;
  const keys = Object.keys(ao);
  return keys.length === Object.keys(bo).length && keys.every((k) => k in bo && sameJson(ao[k], bo[k]));
}
