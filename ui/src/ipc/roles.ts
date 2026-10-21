// V107-R36 The Roles Manager for your own channel, and the usercard's VIP slots. GQL; Rust
// does the work (`app/src/moderation/roles.rs`).

import { invoke } from '@tauri-apps/api/core';

import type { ChannelRole } from './bindings/ChannelRole';
import type { RolesView } from './bindings/RolesView';
import type { VipSlots } from './bindings/VipSlots';

export const rolesIpc = {
  /** Everyone with a role in your channel (`login`, the account's own). */
  view: (account: string, login: string) => invoke<RolesView>('roles_view', { account, login }),
  /** V107-R42 Gives and takes roles for `who` in one change; Rust picks the order and
   *  says the line (`line`). The lists come back; a rejection is the line. */
  set: (account: string, login: string, who: string, give: ChannelRole[], take: ChannelRole[]) =>
    invoke<RolesView>('roles_set', { account, login, who, give, take }),
  /** The VIP slots when `channel` is the account's own; null elsewhere. */
  vipSlots: (account: string, channel: string) => invoke<VipSlots | null>('roles_vip_slots', { account, channel }),
};
