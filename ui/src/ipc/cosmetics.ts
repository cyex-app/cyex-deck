import { invoke } from '@tauri-apps/api/core';

import type { Applied } from './bindings/Applied';
import type { Combo } from './bindings/Combo';
import type { ComboDone } from './bindings/ComboDone';
import type { ComboItem } from './bindings/ComboItem';
import type { ComboOp } from './bindings/ComboOp';
import type { CosmeticsView } from './bindings/CosmeticsView';
import type { Kind } from './bindings/Kind';
import type { Picked } from './bindings/Picked';
import type { SelectError } from './bindings/SelectError';
import type { SignIn } from './bindings/SignIn';

export type { Applied, Combo, ComboDone, ComboItem, ComboOp, CosmeticsView, Kind, Picked, SelectError, SignIn };

export const cosmeticsIpc = {
  /** V1051-02 The account's paints and badges. `fresh` skips Rust's 5 minute copy. */
  view: (account: string, fresh = false) => invoke<CosmeticsView>('cosmetics_view', { account, fresh }),
  /** The user said yes to 7TV for this account. */
  allow: (account: string) => invoke<CosmeticsView>('cosmetics_allow', { account }),
  /** Rejects with a `SelectError` object. */
  select: (account: string, kind: Kind, id: string | null) =>
    invoke<Picked>('cosmetics_select', { account, kind, id }),
  /** V1051-02 7TV's own sign-in window; resolves when it ends. `shared`: in the account's
   * Twitch web profile, so Twitch is signed in already (picked with subscriptions and wallet). */
  signIn: (account: string, shared = false) => invoke<SignIn>('cosmetics_sign_in', { account, shared }),
  /** V108-7K Keeps a 7TV token pasted from the browser. Rejects with the message to show. */
  paste: (account: string, token: string) => invoke<void>('cosmetics_paste', { account, token }),
  /** V108-7K 7tv.app in the browser (and profile) used last. */
  open7tv: () => invoke<void>('cosmetics_open_7tv'),
  signOut: (account: string) => invoke<void>('cosmetics_sign_out', { account }),
  /** Signs out and takes the consent back. */
  withdraw: (account: string) => invoke<void>('cosmetics_withdraw', { account }),
  /** V1051-04 Puts a combo's paint and badge on at once. Rejects with a `SelectError`; `null` takes the part off. */
  apply: (account: string, paint: ComboItem | null, badge: ComboItem | null, name: string | null) =>
    invoke<Applied>('cosmetics_apply', { account, paint, badge, name }),
  /** V1051-04 Rejects with the message to show. */
  combosChange: (account: string, op: ComboOp) => invoke<ComboDone>('combos_change', { account, op }),
};
