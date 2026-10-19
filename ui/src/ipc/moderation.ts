// Moderation (09): the role gate (1.57), running a mod command, the mod view's board
// (1.48) and its queue actions (1.56, M2, M3), mass actions (1.55) and /nuke (M1).

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Board0 } from './bindings/Board0';
import type { ModRole } from './bindings/ModRole';
import type { RoleChange } from './bindings/RoleChange';
import type { RoleEntry } from './bindings/RoleEntry';
import type { MassbanProgress } from './bindings/MassbanProgress';
import type { ModHistory } from './bindings/ModHistory';
import type { NukePreview } from './bindings/NukePreview';
import type { UnbanRequest } from './bindings/UnbanRequest';

export type { ModRole };

export const modIpc = {
  /** 1.57, V102-21 Every open account's role per channel; `onRoles` brings changes. */
  roles: () => invoke<RoleEntry[]>('mod_roles'),
  onRoles: (handler: (list: RoleEntry[]) => void) => listen<RoleEntry[]>('mod:roles', (e) => handler(e.payload)),
  /** V107-M8 Someone was modded, unmodded, made a VIP or not, by us or another mod. */
  onStanding: (handler: (c: RoleChange) => void) => listen<RoleChange>('mod:standing', (e) => handler(e.payload)),
  /** Runs a mod command, e.g. name "ban", args "spammer bots". Returns a line to show. */
  run: (account: string, channel: string, name: string, args: string) =>
    invoke<string | null>('mod_run', { account, channel, name, args }),
  /** M11 Sends a message and pins it for `secs`. */
  pinSend: (account: string, channel: string, text: string, secs: number) =>
    invoke<void>('mod_pin_send', { account, channel, text, secs }),
  channelId: (channel: string) => invoke<string | null>('mod_channel_id', { channel }),
  view: (channelId: string) => invoke<Board0>('mod_view', { channelId }),
  /** V103-15 Twitch's history of the channel's mod actions into its board: the first page
   * once, or with `older` the next one. GQL sign-in only. */
  actionsHistory: (account: string, channel: string, older: boolean) =>
    invoke<void>('mod_actions_history', { account, channel, older }),
  automod: (account: string, channel: string, msgId: string, allow: boolean) =>
    invoke<void>('mod_automod', { account, channel, msgId, allow }),
  unbanRequests: (account: string, channel: string) =>
    invoke<UnbanRequest[]>('mod_unban_requests', { account, channel }),
  resolveUnban: (account: string, channel: string, id: string, approve: boolean, note: string) =>
    invoke<void>('mod_resolve_unban', { account, channel, id, approve, note }),
  history: (account: string, channel: string, targetId: string) =>
    invoke<ModHistory>('mod_history', { account, channel, targetId }),
  addComment: (account: string, channel: string, targetId: string, text: string, shareable: boolean) =>
    invoke<void>('mod_comment_add', { account, channel, targetId, text, shareable }),
  deleteComment: (account: string, channel: string, id: string) =>
    invoke<void>('mod_comment_delete', { account, channel, id }),
  nukePreview: (account: string, channel: string, args: string) =>
    invoke<NukePreview>('mod_nuke_preview', { account, channel, args }),
  massban: (account: string, channel: string, logins: string[], secs: number | null, reason: string) =>
    invoke<number>('mod_massban', { account, channel, logins, secs, reason }),
  stopMassban: (channelId: string) => invoke<void>('mod_massban_stop', { channelId }),
  onBoard: (channelId: string, handler: (b: Board0) => void) =>
    listen<Board0>(`mod:view:${channelId}`, (e) => handler(e.payload)),
  onMassban: (handler: (p: MassbanProgress) => void) =>
    listen<MassbanProgress>('mod:massban', (e) => handler(e.payload)),
};
