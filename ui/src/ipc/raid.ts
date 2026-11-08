// V110-U43 The raid banner over chat.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { RaidChanged } from './bindings/RaidChanged';
import type { RaidGo } from './bindings/RaidGo';
import type { RaidJoinChanged } from './bindings/RaidJoinChanged';
import type { SmRaid } from './bindings/SmRaid';

export const raidIpc = {
  /** The raid counting down from the channel, or null. */
  read: (channelId: string) => invoke<SmRaid | null>('chat_raid', { channelId }),
  /** Follows the channel's raid topic while a chat of it shows. */
  watch: (channelId: string, on: boolean) => invoke<void>('chat_raid_watch', { channelId, on }),
  /** Joins the raid as the chat's account (GQL `joinRaid`). Only ever from the banner's tick. */
  join: (account: string, raidId: string) => invoke<void>('chat_raid_join', { account, raidId }),
  /** Unticked before the raid went: only the pop-up is dropped (Twitch has no leave call we know of). */
  leave: (raidId: string) => invoke<void>('chat_raid_leave', { raidId }),
  /** Raid ids with Join ticked, for a chat that opens mid-raid. */
  ticked: () => invoke<string[]>('chat_raid_ticked'),
  onTick: (handler: (e: RaidJoinChanged) => void) => listen<RaidJoinChanged>('chat:raid-join', (e) => handler(e.payload)),
  /** Main window: a ticked raid went. */
  onGo: (handler: (e: RaidGo) => void) => listen<RaidGo>('chat:raid-go', (e) => handler(e.payload)),
  onChange: (handler: (e: RaidChanged) => void) => listen<RaidChanged>('chat:raid', (e) => handler(e.payload)),
};
