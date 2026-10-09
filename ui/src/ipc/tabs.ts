import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChannelInfo } from './bindings/ChannelInfo';
import type { TabOp } from './bindings/TabOp';
import type { TabsApplied } from './bindings/TabsApplied';

export const tabsIpc = {
  /** The new list, and the id of a new tab or group if the edit made one. */
  apply: (op: TabOp) => invoke<TabsApplied>('tabs_apply', { op }),
  /** 1.14 Channels matching a name, user id or twitch.tv link. */
  lookup: (input: string) => invoke<ChannelInfo[]>('tabs_lookup', { input }),
  live: () => invoke<ChannelInfo[]>('tabs_live'),
  onLive: (handler: (channels: ChannelInfo[]) => void) =>
    listen<ChannelInfo[]>('tabs:live', (e) => handler(e.payload)),
  /** 1.11 `null` when it can't be told (no GQL sign-in). */
  following: (channelId: string, account: string) =>
    invoke<boolean | null>('channel_following', { channelId, account }),
  follow: (channelId: string, account: string, follow: boolean) =>
    invoke<void>('channel_follow', { channelId, account, follow }),
};
