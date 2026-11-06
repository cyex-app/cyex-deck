import { invoke } from '@tauri-apps/api/core';

import type { CyexEquipped } from './bindings/CyexEquipped';
import type { CyexHistory } from './bindings/CyexHistory';
import type { CyexMe } from './bindings/CyexMe';
import type { CyexPics } from './bindings/CyexPics';
import type { CyexView } from './bindings/CyexView';

export type { CyexEquipped, CyexHistory, CyexMe, CyexPics, CyexView };

/** V109-B02 cyex cosmetics of one account (api.cyex.app, with the account's own Twitch sign-in). */
export const cyexIpc = {
  view: (account: string) => invoke<CyexView>('cyex_view', { account }),
  /** Rejects with the message to show. */
  equip: (account: string, main: string | null, custom: boolean) =>
    invoke<CyexEquipped>('cyex_equip', { account, main, custom }),
  /** What the code gave, as a line. Rejects with the message to show. */
  redeem: (account: string, code: string) => invoke<string>('cyex_redeem', { account, code }),
  /** V109-B06 The Supporter+ uploads. Rejects with the message to show. */
  history: (account: string) => invoke<CyexHistory>('cyex_badge_history', { account }),
  /** V109-B06 An earlier approved upload goes live. Rejects with the message to show. */
  restore: (account: string, id: number) => invoke<null>('cyex_badge_restore', { account, id }),
  /** V109-B07 An own upload's preview (PNG bytes): `<img>` can't send the session. */
  preview: (account: string, url: string) => invoke<ArrayBuffer>('cyex_badge_preview', { account, url }),
};
