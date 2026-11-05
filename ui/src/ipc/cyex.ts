import { invoke } from '@tauri-apps/api/core';

import type { CyexEquipped } from './bindings/CyexEquipped';
import type { CyexMe } from './bindings/CyexMe';
import type { CyexPics } from './bindings/CyexPics';
import type { CyexView } from './bindings/CyexView';

export type { CyexEquipped, CyexMe, CyexPics, CyexView };

/** V109-B02 cyex cosmetics of one account (api.cyex.app, with the account's own Twitch sign-in). */
export const cyexIpc = {
  view: (account: string) => invoke<CyexView>('cyex_view', { account }),
  /** Rejects with the message to show. */
  equip: (account: string, main: string | null, custom: boolean) =>
    invoke<CyexEquipped>('cyex_equip', { account, main, custom }),
  /** What the code gave, as a line. Rejects with the message to show. */
  redeem: (account: string, code: string) => invoke<string>('cyex_redeem', { account, code }),
};
