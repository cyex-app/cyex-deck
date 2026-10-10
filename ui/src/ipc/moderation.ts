// Moderation (09): the role gate (1.57), running a mod command, the mod view's board
// (1.48) and its queue actions (1.56, M2, M3), mass actions (1.55) and /nuke (M1).

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Board0 } from './bindings/Board0';
import type { Chatters } from './bindings/Chatters';
import type { MassbanProgress } from './bindings/MassbanProgress';
import type { ModHistory } from './bindings/ModHistory';
import type { NukePreview } from './bindings/NukePreview';
import type { UnbanRequest } from './bindings/UnbanRequest';

export type ModRole = 'broadcaster' | 'moderator';

export const modIpc = {
  /** 1.57 The account's role in a channel, or null when it can't moderate. */
  role: (account: string, channel: string) => invoke<ModRole | null>('mod_role', { account, channel }),
  /** Which open account moderates a channel, for the tab's shield. */
  moderating: (channel: string) => invoke<string | null>('mod_moderating', { channel }),
  /** Runs a mod command, e.g. name "ban", args "spammer bots". Returns a line to show. */
  run: (account: string, channel: string, name: string, args: string) =>
    invoke<string | null>('mod_run', { account, channel, name, args }),
  channelId: (channel: string) => invoke<string | null>('mod_channel_id', { channel }),
  view: (channelId: string) => invoke<Board0>('mod_view', { channelId }),
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
  chatters: (account: string, channel: string) => invoke<Chatters>('mod_chatters', { account, channel }),
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
