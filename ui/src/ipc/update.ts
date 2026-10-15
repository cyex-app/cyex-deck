// V105-20 Update checks and the in-app update.

import { Channel, invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { CheckResult } from './bindings/CheckResult';
import type { Update } from './bindings/Update';
import type { UpdateProgress } from './bindings/UpdateProgress';

export const updateIpc = {
  state: () => invoke<Update | null>('update_state'),
  /** "Check now": ignores the skipped version. */
  check: () => invoke<CheckResult>('update_check'),
  skip: (version: string) => invoke<void>('update_skip', { version }),
  /** Downloads, checks the file and restarts into the new version. */
  install: (onEvent: Channel<UpdateProgress>) => invoke<void>('update_install', { onEvent }),
  openNotes: () => invoke<void>('update_open_notes'),
  onAvailable: (cb: (u: Update | null) => void): Promise<UnlistenFn> =>
    listen<Update | null>('update://available', (e) => cb(e.payload)),
};
