// Whispers (1.26): the default account's threads and conversations.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { WhisperConversation } from './bindings/WhisperConversation';
import type { WhisperEdit } from './bindings/WhisperEdit';
import type { WhisperLine } from './bindings/WhisperLine';
import type { WhisperPage } from './bindings/WhisperPage';
import type { WhisperState } from './bindings/WhisperState';

export const whispersIpc = {
  state: () => invoke<WhisperState>('whispers_state'),
  /** Loads the next page of threads; the list comes back through `onChanged`. */
  more: () => invoke<void>('whispers_more'),
  open: (login: string) => invoke<WhisperConversation>('whispers_open', { login }),
  older: (threadId: string, cursor: string) => invoke<WhisperPage>('whispers_older', { threadId, cursor }),
  send: (to: string, text: string) => invoke<void>('whispers_send', { to, text }),
  read: (threadId: string) => invoke<void>('whispers_read', { threadId }),
  edit: (threadId: string, edit: WhisperEdit) => invoke<void>('whispers_edit', { threadId, edit }),
  readAll: () => invoke<void>('whispers_read_all'),
  block: (login: string) => invoke<void>('whispers_block', { login }),
  blockStrangers: (on: boolean) => invoke<void>('whispers_block_strangers', { on }),
  /** The thread a focused view shows, so it gets no unread count or alert. */
  viewing: (threadId: string | null) => invoke<void>('whispers_viewing', { threadId }),
  onChanged: (cb: (s: WhisperState) => void): Promise<UnlistenFn> =>
    listen<WhisperState>('whispers:changed', (e) => cb(e.payload)),
  onLine: (cb: (l: WhisperLine) => void): Promise<UnlistenFn> =>
    listen<WhisperLine>('whispers:line', (e) => cb(e.payload)),
  /** A line was edited or deleted: reload the thread if it's open. */
  onRefresh: (cb: (threadId: string) => void): Promise<UnlistenFn> =>
    listen<{ threadId: string }>('whispers:refresh', (e) => cb(e.payload.threadId)),
};
