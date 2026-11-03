// V108-N04 The hype train bar over chat.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { HypeChanged } from './bindings/HypeChanged';
import type { HypeTrain } from './bindings/HypeTrain';
import type { HypeUnlock } from './bindings/HypeUnlock';

export const hypeIpc = {
  /** The channel's running train, or null. */
  read: (login: string) => invoke<HypeTrain | null>('chat_hype_train', { login }),
  /** The channel's last train, ended or not: the closing stats. */
  last: (login: string) => invoke<HypeTrain | null>('chat_hype_train', { login, ended: true }),
  /** Follows the train's topic while a chat of a live channel shows the bar; with the chat's
   * account (user id), its hype train rewards too (V108-C35). */
  watch: (channelId: string, account: string | null, on: boolean) => invoke<void>('chat_hype_watch', { channelId, account, on }),
  onChange: (handler: (e: HypeChanged) => void) => listen<HypeChanged>('chat:hype-train', (e) => handler(e.payload)),
  /** V108-C35 Emotes a hype train gave an account. */
  onUnlock: (handler: (e: HypeUnlock) => void) => listen<HypeUnlock>('chat:hype-unlock', (e) => handler(e.payload)),
};
