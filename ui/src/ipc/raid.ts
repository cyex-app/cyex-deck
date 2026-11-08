// V110-U43 The raid banner over chat.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { RaidChanged } from './bindings/RaidChanged';
import type { SmRaid } from './bindings/SmRaid';

export const raidIpc = {
  /** The raid counting down from the channel, or null. */
  read: (channelId: string) => invoke<SmRaid | null>('chat_raid', { channelId }),
  /** Follows the channel's raid topic while a chat of it shows. */
  watch: (channelId: string, on: boolean) => invoke<void>('chat_raid_watch', { channelId, on }),
  /** Joins the raid as the chat's account. Only ever from the banner's button. */
  join: (account: string, raidId: string) => invoke<void>('chat_raid_join', { account, raidId }),
  onChange: (handler: (e: RaidChanged) => void) => listen<RaidChanged>('chat:raid', (e) => handler(e.payload)),
};
