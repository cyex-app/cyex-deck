// V110-F73 One thread after a whisper came in, merged into the list Rust sent before.

import type { WhisperState } from '../../ipc/bindings/WhisperState';
import type { WhisperThreadChange } from '../../ipc/bindings/WhisperThreadChange';

/** Puts the thread in (new or replaced), drops the cut ones, keeps newest first. */
export function applyChange(state: WhisperState, change: WhisperThreadChange): WhisperState {
  const gone = new Set(change.dropped);
  const threads = state.threads.filter((t) => t.id !== change.thread.id && !gone.has(t.id));
  threads.push(change.thread);
  // Stable, like Rust's sort: equal times keep their order.
  threads.sort((a, b) => b.lastTs - a.lastTs);
  return { ...state, threads, unread: change.unread };
}
