// V107-M1 History goes on screen a piece per frame: a chat shown again gets its older lines
// back in chunks, and two chunks landing in one frame drew 100 lines at once (a 90 ms frame).
// Queued chunks arrive newest first; each holds its lines oldest first.

export const PIECE = 25;

type Item = { id: string };

/** The newest `n` lines still queued, taken out of the queue. */
export function nextPiece<T extends Item>(queue: T[][], n = PIECE): T[] {
  while (queue.length && !queue[0].length) queue.shift();
  const chunk = queue[0];
  if (!chunk) return [];
  const piece = chunk.splice(Math.max(0, chunk.length - n));
  if (!chunk.length) queue.shift();
  return piece;
}

/** `piece` above `list`, without lines it has already. */
export function prepend<T extends Item>(list: T[], piece: T[]): T[] {
  const known = new Set(list.map((m) => m.id));
  const fresh = piece.filter((m) => !known.has(m.id));
  return fresh.length ? [...fresh, ...list] : list;
}
