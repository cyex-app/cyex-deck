// V107-R23 Whether a user's lines in a room differ from what the card showed (a new line, or
// one marked deleted), so the card reads them again only then.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

/** The lines in `list` that `userId` wrote. */
export function theirLines(list: readonly ChatMessage[], userId: string): ChatMessage[] {
  return list.filter((m) => !m.notice && m.sender?.id === userId);
}

/** `true` when `next` has a line `prev` lacks, or a changed one (rooms replace edited lines). */
export function changedLines(prev: readonly ChatMessage[], next: readonly ChatMessage[]): boolean {
  if (prev.length !== next.length) return true;
  return next.some((m, i) => m !== prev[i]);
}
