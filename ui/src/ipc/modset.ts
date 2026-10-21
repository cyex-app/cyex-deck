// V107-R38 Your channel's moderation settings (stream manager). GQL only; each call answers
// with everything read back.

import { invoke } from '@tauri-apps/api/core';

import type { ModsetChange } from './bindings/ModsetChange';
import type { ModsetSettings } from './bindings/ModsetSettings';

export const modsetIpc = {
  read: () => invoke<ModsetSettings>('modset_read'),
  set: (change: ModsetChange) => invoke<ModsetSettings>('modset_set', { change }),
  addTerm: (permitted: boolean, text: string) => invoke<ModsetSettings>('modset_add_term', { permitted, text }),
  removeTerm: (permitted: boolean, id: string) => invoke<ModsetSettings>('modset_remove_term', { permitted, id }),
};
