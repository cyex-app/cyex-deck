// V110-F83 Which player a key acts on: the one used last if it is on screen, else the first that is.

export function pickShown<T extends { visible?(): boolean }>(last: T | null, list: readonly T[]): T | null {
  if (last?.visible?.()) return last;
  return list.find((p) => p.visible?.()) ?? null;
}
