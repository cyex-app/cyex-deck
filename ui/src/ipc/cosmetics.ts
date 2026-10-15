import { invoke } from '@tauri-apps/api/core';

import type { CosmeticsView } from './bindings/CosmeticsView';
import type { Kind } from './bindings/Kind';
import type { Picked } from './bindings/Picked';
import type { SelectError } from './bindings/SelectError';
import type { SignIn } from './bindings/SignIn';

export type { CosmeticsView, Kind, Picked, SelectError, SignIn };

export const cosmeticsIpc = {
  /** V1051-02 The account's paints and badges. `fresh` skips Rust's 5 minute copy. */
  view: (account: string, fresh = false) => invoke<CosmeticsView>('cosmetics_view', { account, fresh }),
  /** The user said yes to 7TV for this account. */
  allow: (account: string) => invoke<CosmeticsView>('cosmetics_allow', { account }),
  /** Rejects with a `SelectError` object. */
  select: (account: string, kind: Kind, id: string | null) =>
    invoke<Picked>('cosmetics_select', { account, kind, id }),
  /** V1051-02 7TV's own sign-in window; resolves when it ends. */
  signIn: (account: string) => invoke<SignIn>('cosmetics_sign_in', { account }),
  signOut: (account: string) => invoke<void>('cosmetics_sign_out', { account }),
  /** Signs out and takes the consent back. */
  withdraw: (account: string) => invoke<void>('cosmetics_withdraw', { account }),
};
