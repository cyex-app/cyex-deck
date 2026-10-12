// Settings editors' checks (F3 filters, 1.8 commands), S1 menus, 1.36 usercard extras.

import { invoke } from '@tauri-apps/api/core';

import type { BadgeInfo } from './bindings/BadgeInfo';
import type { CommandLineError } from './bindings/CommandLineError';
import type { FilterName } from './bindings/FilterName';
import type { FollowAge } from './bindings/FollowAge';
import type { LogPage } from './bindings/LogPage';
import type { MenuContext } from './bindings/MenuContext';
import type { Menus } from './bindings/Menus';
import type { NameChange } from './bindings/NameChange';
import type { SubStatus } from './bindings/SubStatus';
import type { UiAction } from './bindings/UiAction';

export const rulesIpc = {
  /** Why a filter doesn't read, or null. */
  checkFilter: (expr: string) => invoke<string | null>('filter_check', { expr }),
  filterNames: () => invoke<FilterName[]>('filter_names'),
  checkCommands: (lines: string[]) => invoke<CommandLineError[]>('commands_check', { lines }),
  menus: () => invoke<Menus>('chat_menus'),
};

export const usercardIpc = {
  subStatus: (user: string, channel: string) => invoke<SubStatus>('user_sub_status', { user, channel }),
  followAge: (channel: string, userId: string) => invoke<FollowAge>('user_follow_age', { channel, userId }),
  nameHistory: (userId: string) => invoke<NameChange[]>('user_name_history', { userId }),
  /** V102-13 Every Twitch badge they own in the channel. */
  badges: (user: string, channel: string) => invoke<BadgeInfo[]>('user_badges', { user, channel }),
  logs: (channel: string, userId: string, user: string, offset: number) =>
    invoke<LogPage>('user_logs', { channel, userId, user, offset }),
  /** M8 Twitch's mod logs, for a mod where logs.cyex.app has none. */
  modLogs: (account: string, channel: string, userId: string, cursor: string | null) =>
    invoke<LogPage>('user_mod_logs', { account, channel, userId, cursor }),
  /** S1 A usercard button, run in an open view of `channel`. */
  run: (channel: string, command: string, context: MenuContext) =>
    invoke<UiAction | null>('usercard_run', { channel, command, context }),
  blocked: (userId: string) => invoke<boolean>('user_blocked', { userId }),
};
