import { invoke } from '@tauri-apps/api/core';

import type { CyextvMenuOptions } from './bindings/CyextvMenuOptions';
import type { TvBadge } from './bindings/TvBadge';
import type { TvCosmetics } from './bindings/TvCosmetics';
import type { TvKind } from './bindings/TvKind';
import type { TvOwned } from './bindings/TvOwned';
import type { TvPaint } from './bindings/TvPaint';

export type { CyextvMenuOptions, TvBadge, TvCosmetics, TvKind, TvOwned, TvPaint };

/** V112-14/16 cyex.tv (api.cyex.tv) per account: its own sign-in, and the cosmetics it owns there. */
export const cyextvIpc = {
  /** cyex.tv's sign-in in the browser. Resolves with the cyex.tv name; rejects with the line to show ('' when cancelled). */
  signIn: (account: string) => invoke<string>('cyextv_sign_in', { account }),
  signInCancel: () => invoke<void>('cyextv_sign_in_cancel'),
  signOut: (account: string) => invoke<void>('cyextv_sign_out', { account }),
  cosmetics: (account: string) => invoke<TvCosmetics>('cyextv_cosmetics', { account }),
  /** `null` takes it off. Rejects with the line to show. */
  select: (account: string, kind: TvKind, id: string | null) => invoke<TvOwned>('cyextv_select', { account, kind, id }),
  /** V112-17 Whether the emote menu offers "Add to cyex.tv" for this chat's channel. */
  emoteOptions: (account: string, login: string, emoteId: string) =>
    invoke<CyextvMenuOptions>('cyextv_emote_options', { account, login, emoteId }),
  /** Adds it under its own name; resolves with the line to show, rejects with the one to show. */
  emoteAdd: (account: string, login: string, emoteId: string) =>
    invoke<string>('cyextv_emote_add', { account, login, emoteId }),
};
