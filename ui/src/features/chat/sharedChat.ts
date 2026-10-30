// V108-N12 Shared Chat in the view. Lines from another channel carry `source` (Rust leaves it
// off this channel's own lines). "Only this channel" hides them from the list; they stay in
// the room, so switching it off shows them again.

import type { ChatMessage } from '../../ipc/bindings/ChatMessage';

/** A channel whose lines showed up this recently is still in the shared chat (the same
 *  10 minutes Rust uses for 7TV presence). */
export const SHARED_SEEN_MS = 10 * 60 * 1000;

type Lined = Pick<ChatMessage, 'ts' | 'source'>;

/** Another channel's line came in within the last `SHARED_SEEN_MS`. */
export function inSharedSession(messages: readonly Lined[], now: number): boolean {
  for (let i = messages.length - 1; i >= 0; i--) {
    const m = messages[i];
    if (m.ts < now - SHARED_SEEN_MS) return false;
    if (m.source) return true;
  }
  return false;
}

/** The lines left when only this channel's show. */
export function onlyHere<T extends Pick<ChatMessage, 'source'>>(messages: readonly T[]): T[] {
  return messages.filter((m) => !m.source);
}

/** How many lines the filter hides, for the tooltip. */
export function hiddenCount(messages: readonly Pick<ChatMessage, 'source'>[]): number {
  let n = 0;
  for (const m of messages) if (m.source) n++;
  return n;
}

/** "A", "A and B", "A, B and C". */
export function listNames(names: readonly string[]): string {
  if (names.length < 2) return names[0] ?? '';
  return `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`;
}
