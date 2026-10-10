// Subscriptions (1.28) and Twitch's own pages in the app (3.8, 1.27).

import { invoke } from '@tauri-apps/api/core';

import type { ChannelSub } from './bindings/ChannelSub';
import type { MySubs } from './bindings/MySubs';

/** Twitch pages the app opens in its Twitch window. */
export type TwitchPage = 'wallet' | 'subscriptions' | 'subscribe' | 'gift';

export const subsIpc = {
  mine: (account: string) => invoke<MySubs>('subs_mine', { account }),
  channel: (account: string, login: string) => invoke<ChannelSub | null>('subs_channel', { account, login }),
  shareResub: (account: string, login: string, tokenId: string, message: string, streak: boolean) =>
    invoke<void>('subs_share_resub', { account, login, tokenId, message, streak }),
  prime: (account: string, channelId: string) => invoke<void>('subs_prime', { account, channelId }),
  stopRenewal: (account: string, subId: string) => invoke<void>('subs_stop_renewal', { account, subId }),
  /** Opens a Twitch page in the account's Twitch window. */
  page: (account: string, kind: TwitchPage, login: string | null = null) =>
    invoke<void>('twitch_page', { account, kind, login }),
  /** Opens any twitch.tv address there. */
  open: (account: string, url: string) => invoke<void>('twitch_open', { account, url }),
  /** Clears the account's sign-in on Twitch's pages. */
  signOut: (account: string) => invoke<void>('twitch_sign_out', { account }),
};
