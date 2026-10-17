import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChannelInfo } from './bindings/ChannelInfo';
import type { RoomUnread } from './bindings/RoomUnread';
import type { TabOp } from './bindings/TabOp';
import type { TabsApplied } from './bindings/TabsApplied';

export const tabsIpc = {
  /** The new list, and the id of a new tab or group if the edit made one. */
  apply: (op: TabOp) => invoke<TabsApplied>('tabs_apply', { op }),
  /** 1.14 Channels matching a name, user id or twitch.tv link. */
  lookup: (input: string) => invoke<ChannelInfo[]>('tabs_lookup', { input }),
  /** U10 Reopens what the last close took; `made` is the tab it shows. */
  reopen: () => invoke<TabsApplied>('tabs_reopen'),
  reopenable: () => invoke<boolean>('tabs_reopenable'),
  onReopenable: (handler: (any: boolean) => void) => listen<boolean>('tabs:reopenable', (e) => handler(e.payload)),
  /** U9 Chats with news nobody has seen. */
  unread: () => invoke<RoomUnread[]>('tabs_unread'),
  onUnread: (handler: (rooms: RoomUnread[]) => void) =>
    listen<RoomUnread[]>('chat:unread', (e) => handler(e.payload)),
  live: () => invoke<ChannelInfo[]>('tabs_live'),
  onLive: (handler: (channels: ChannelInfo[]) => void) =>
    listen<ChannelInfo[]>('tabs:live', (e) => handler(e.payload)),
  /** V106-25 Mutes or unmutes everything in a tab (not saved). Resolves with the muted tabs. */
  mute: (tab: string, on: boolean) => invoke<string[]>('tabs_mute', { tab, on }),
  muted: () => invoke<string[]>('tabs_muted'),
  onMuted: (h: (ids: string[]) => void) => listen<string[]>('tabs:muted', (e) => h(e.payload)),
  /** 1.11 `null` when it can't be told (no GQL sign-in). */
  following: (channelId: string, account: string) =>
    invoke<boolean | null>('channel_following', { channelId, account }),
  follow: (channelId: string, account: string, follow: boolean) =>
    invoke<void>('channel_follow', { channelId, account, follow }),
};
