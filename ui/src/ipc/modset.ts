// V107-R38 Your channel's moderation settings (stream manager). GQL only; each call answers
// with everything read back.

import { invoke } from '@tauri-apps/api/core';

import type { ModsetChange } from './bindings/ModsetChange';
import type { ModsetSettings } from './bindings/ModsetSettings';

/** V109-C04 `account`: whose channel (the creator dashboard's pick); unset, the stream manager's. */
export const modsetIpc = {
  read: (account?: string | null) => invoke<ModsetSettings>('modset_read', { account }),
  set: (change: ModsetChange, account?: string | null) => invoke<ModsetSettings>('modset_set', { change, account }),
  /** `have`: the terms shown, checked first so no read runs before. */
  addTerm: (permitted: boolean, text: string, have: string[], account?: string | null) =>
    invoke<ModsetSettings>('modset_add_term', { permitted, text, have, account }),
  removeTerm: (permitted: boolean, id: string, account?: string | null) =>
    invoke<ModsetSettings>('modset_remove_term', { permitted, id, account }),
};
