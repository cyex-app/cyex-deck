// V107-R40 Stream manager › Community: your followers and the roles you hold elsewhere.
// GQL, the way Twitch's dashboard reads them; Rust does the work.

import { invoke } from '@tauri-apps/api/core';

import type { CmAssigned } from './bindings/CmAssigned';
import type { CmFollowers } from './bindings/CmFollowers';
import type { CmRole } from './bindings/CmRole';

export const cmIpc = {
  /** A page of followers after `cursor` (null: the first), newest or oldest first. */
  followers: (cursor: string | null, newest: boolean) => invoke<CmFollowers>('cm_followers', { cursor, newest }),
  assigned: () => invoke<CmAssigned[]>('cm_assigned'),
  /** Leaves your own role in a channel. */
  stepDown: (role: CmRole, channelId: string) => invoke<void>('cm_step_down', { role, channelId }),
};
