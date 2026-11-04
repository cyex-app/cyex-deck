// V108-C40 The chat account's own standing in a channel: the locked chat box.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { ChatStanding } from './bindings/ChatStanding';
import type { StandingChanged } from './bindings/StandingChanged';

export const standingIpc = {
  read: (account: string, channelId: string) => invoke<ChatStanding>('chat_standing', { account, channelId }),
  /** Only from the unban form's Send. */
  requestUnban: (account: string, channelId: string, message: string) =>
    invoke<void>('chat_request_unban', { account, channelId, message }),
  /** Only from Acknowledge. */
  acknowledge: (account: string, channelId: string) => invoke<void>('chat_ack_warning', { account, channelId }),
  /** Follows the account's own moderation topic while one of its chat boxes is open. */
  watch: (account: string, on: boolean) => invoke<void>('chat_standing_watch', { account, on }),
  onChange: (handler: (e: StandingChanged) => void) => listen<StandingChanged>('chat:standing', (e) => handler(e.payload)),
};
