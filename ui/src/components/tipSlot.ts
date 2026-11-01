// V108-C25 What arrives for a tip after it was asked for (a link's card, a new title) waits
// here until the tip shows, and belongs to the element it was asked for: an answer for
// another element, or one that comes after the pointer left, never lands.

export interface TipSlot<K, V> {
  /** `key` is hovered now. The same key again keeps what's already waiting. */
  claim(key: K, initial?: V): void;
  release(): void;
  /** Keeps `value` for `key`; false when `key` isn't the hovered one. */
  put(key: K, value: V): boolean;
  /** What's waiting for `key`, if it's still the hovered one. */
  read(key: K): V | undefined;
  owner(): K | null;
}

export function tipSlot<K, V>(): TipSlot<K, V> {
  let owner: K | null = null;
  let value: V | undefined;
  return {
    claim(key, initial) {
      if (owner === key && value !== undefined) return;
      owner = key;
      value = initial;
    },
    release() {
      owner = null;
      value = undefined;
    },
    put(key, v) {
      if (owner !== key) return false;
      value = v;
      return true;
    },
    read: (key) => (owner === key ? value : undefined),
    owner: () => owner,
  };
}
