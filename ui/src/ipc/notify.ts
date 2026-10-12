// The bell (1.26), Twitch profile and notification settings (2.2), and what a toast click
// asks the main window to open.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { BellItem } from './bindings/BellItem';
import type { BellList } from './bindings/BellList';
import type { BellPage } from './bindings/BellPage';
import type { BellState } from './bindings/BellState';
import type { NotifyOpen } from './bindings/NotifyOpen';
import type { TwitchNotify } from './bindings/TwitchNotify';
import type { TwitchProfile } from './bindings/TwitchProfile';

/** A live change to the bell list. */
export type BellLive =
  | { kind: 'created'; list: BellList; item: BellItem }
  | { kind: 'updated'; item: BellItem }
  | { kind: 'deleted' | 'read'; ids: string[] }
  | { kind: 'allRead' };

export const bellIpc = {
  state: () => invoke<BellState>('bell_state'),
  list: (list: BellList, cursor: string | null = null) => invoke<BellPage>('bell_list', { list, cursor }),
  /** The bell was opened: clears the badge on Twitch too. */
  seen: () => invoke<void>('bell_seen'),
  read: (ids: string[]) => invoke<void>('bell_read', { ids }),
  readAll: (list: BellList) => invoke<void>('bell_read_all', { list }),
  remove: (id: string) => invoke<void>('bell_delete', { id }),
  onUnseen: (cb: (s: BellState) => void): Promise<UnlistenFn> =>
    listen<BellState>('bell:unseen', (e) => cb(e.payload)),
  onLive: (cb: (l: BellLive) => void): Promise<UnlistenFn> => listen<BellLive>('bell:live', (e) => cb(e.payload)),
};

export const profileIpc = {
  get: (account: string) => invoke<TwitchProfile>('profile_get', { account }),
  save: (account: string, displayName: string, bio: string) =>
    invoke<void>('profile_save', { account, displayName, bio }),
  notify: (account: string, cursor: string | null = null) =>
    invoke<TwitchNotify>('twitch_notify_get', { account, cursor }),
  setSwitch: (account: string, category: string, platform: string, on: boolean) =>
    invoke<void>('twitch_notify_switch', { account, category, platform, on }),
  setChannel: (account: string, channelId: string, state: string) =>
    invoke<void>('twitch_notify_channel', { account, channelId, state }),
};

export const notifyEvents = {
  /** A toast was clicked. Main window only. */
  onOpen: (cb: (o: NotifyOpen) => void): Promise<UnlistenFn> =>
    listen<NotifyOpen>('notify:open', (e) => cb(e.payload)),
  /** Play the app's alert sound. Main window only. */
  onSound: (cb: () => void): Promise<UnlistenFn> => listen('notify:sound', () => cb()),
};
