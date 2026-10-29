// Subscriptions (1.28) and Twitch's own pages in the app (3.8, 1.27).

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChannelSub } from './bindings/ChannelSub';
import type { MySubs } from './bindings/MySubs';
import type { Streak } from './bindings/Streak';
import type { StreakChanged } from './bindings/StreakChanged';

/** Twitch pages the app opens in its Twitch window. */
export type TwitchPage = 'wallet' | 'subscriptions' | 'bits' | 'subscribe' | 'gift';

export const subsIpc = {
  mine: (account: string) => invoke<MySubs>('subs_mine', { account }),
  channel: (account: string, login: string) => invoke<ChannelSub | null>('subs_channel', { account, login }),
  shareResub: (account: string, login: string, tokenId: string, message: string, streak: boolean) =>
    invoke<void>('subs_share_resub', { account, login, tokenId, message, streak }),
  /** V108-N23 The watch streak the account can share in a channel, if any. */
  streak: (account: string, login: string) => invoke<Streak>('subs_streak', { account, login }),
  /** V108-N23 Posts it to chat as Twitch's own notice. Only from a click. */
  shareStreak: (account: string, channelId: string, milestoneId: string) =>
    invoke<void>('subs_share_streak', { account, channelId, milestoneId }),
  /** V108-N23 Follows the milestones topic while a prompt could show. */
  watchStreak: (account: string, on: boolean) => invoke<void>('subs_watch_streak', { account, on }),
  onStreak: (handler: (e: StreakChanged) => void) => listen<StreakChanged>('subs:streak', (e) => handler(e.payload)),
  prime: (account: string, channelId: string) => invoke<void>('subs_prime', { account, channelId }),
  stopRenewal: (account: string, subId: string) => invoke<void>('subs_stop_renewal', { account, subId }),
  /** Opens a Twitch page in the account's Twitch window. */
  page: (account: string, kind: TwitchPage, login: string | null = null) =>
    invoke<void>('twitch_page', { account, kind, login }),
  /** Opens any twitch.tv address there. */
  open: (account: string, url: string) => invoke<void>('twitch_open', { account, url }),
  /** Clears the account's sign-in on Twitch's pages. */
  signOut: (account: string) => invoke<void>('twitch_sign_out', { account }),
  /** V108-04 Twitch's login in the account's Twitch window; `true` once signed in there. */
  connect: (account: string) => invoke<boolean>('twitch_connect', { account }),
  /** V108-04 The accounts whose Twitch pages are signed in. */
  connected: () => invoke<string[]>('twitch_connected'),
  onConnected: (handler: (accounts: string[]) => void) =>
    listen<string[]>('twitch-pages:changed', (e) => handler(e.payload)),
};
