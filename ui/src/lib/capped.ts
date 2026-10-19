// L1 Maps that live as long as the window but grow with what the user opens (channels,
// tiles): kept to a cap, oldest use out first. A Map keeps insertion order, so a use
// moves the key to the end.

/** Sets `key` as the newest entry, then drops the oldest past `max`. */
export function setCapped<K, V>(map: Map<K, V>, key: K, value: V, max: number): void {
  map.delete(key);
  map.set(key, value);
  dropOldest(map, max);
}

/** The value of `key`, marked as used now. */
export function useCapped<K, V>(map: Map<K, V>, key: K): V | undefined {
  if (!map.has(key)) return undefined;
  const v = map.get(key) as V;
  map.delete(key);
  map.set(key, v);
  return v;
}

/** Drops the oldest entries until at most `max` are left. */
export function dropOldest<K, V>(map: Map<K, V>, max: number): void {
  let extra = map.size - Math.max(0, max);
  if (extra <= 0) return;
  for (const k of map.keys()) {
    map.delete(k);
    if (--extra <= 0) break;
  }
}

/** Drops the entries `stale` says are too old. */
export function dropStale<K, V>(map: Map<K, V>, stale: (v: V) => boolean): void {
  for (const [k, v] of map) if (stale(v)) map.delete(k);
}

/** Keeps only the keys `keep` still wants (e.g. tiles that still exist). */
export function keepOnly<K, V>(map: Map<K, V>, keep: (k: K) => boolean): void {
  for (const k of [...map.keys()]) if (!keep(k)) map.delete(k);
}

/** A Map that keeps at most `max` entries; each set makes its key the newest. */
export class CappedMap<K, V> extends Map<K, V> {
  private readonly max: number;

  constructor(max: number) {
    super();
    this.max = max;
  }

  override set(key: K, value: V): this {
    super.delete(key);
    super.set(key, value);
    dropOldest(this, this.max);
    return this;
  }
}
