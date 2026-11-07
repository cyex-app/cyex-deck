// V108-C40 The chat account's own standing in a channel: the locked chat box.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { KnownStanding } from './bindings/KnownStanding';
import type { StandingChanged } from './bindings/StandingChanged';

export const standingIpc = {
  /** Asks Twitch; the answer is folded into what the app knew (V109-F79). */
  read: (account: string, channelId: string) => invoke<KnownStanding>('chat_standing', { account, channelId }),
  /** V109-F79 What the app knows for this account and channel, without asking Twitch. */
  known: (account: string, channelId: string) => invoke<KnownStanding>('chat_standing_known', { account, channelId }),
  /** Only from the unban form's Send. */
  requestUnban: (account: string, channelId: string, message: string) =>
    invoke<void>('chat_request_unban', { account, channelId, message }),
  /** Only from Acknowledge. */
  acknowledge: (account: string, channelId: string) => invoke<void>('chat_ack_warning', { account, channelId }),
  /** Follows the account's own moderation topic while one of its chat boxes is open. */
  watch: (account: string, on: boolean) => invoke<void>('chat_standing_watch', { account, on }),
  onChange: (handler: (e: StandingChanged) => void) => listen<StandingChanged>('chat:standing', (e) => handler(e.payload)),
};
