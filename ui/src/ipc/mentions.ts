// V104-01, V104-02, V106-33 Highlights: pattern checks, the firehose's status, sound files.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { GlobalStatus } from './bindings/GlobalStatus';
import type { MentionCheck } from './bindings/MentionCheck';
import type { PhraseRule } from './bindings/PhraseRule';

export const mentionsIpc = {
  check: (list: PhraseRule[]) => invoke<MentionCheck[]>('mentions_check', { list }),
  globalStatus: () => invoke<GlobalStatus>('mentions_global_status'),
  onGlobalStatus: (cb: (s: GlobalStatus) => void): Promise<UnlistenFn> =>
    listen<GlobalStatus>('mentions:global', (e) => cb(e.payload)),
  pickSound: () => invoke<string | null>('mentions_pick_sound'),
  /** The file's bytes; only files a mention names. */
  sound: (path: string) => invoke<ArrayBuffer>('mentions_sound', { path }),
};
