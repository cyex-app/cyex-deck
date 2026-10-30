// V108-N04 The hype train bar over chat.

import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { HypeChanged } from './bindings/HypeChanged';
import type { HypeTrain } from './bindings/HypeTrain';

export const hypeIpc = {
  /** The channel's running train, or null. */
  read: (login: string) => invoke<HypeTrain | null>('chat_hype_train', { login }),
  /** Follows the train's topic while a chat of a live channel shows the bar. */
  watch: (channelId: string, on: boolean) => invoke<void>('chat_hype_watch', { channelId, on }),
  onChange: (handler: (e: HypeChanged) => void) => listen<HypeChanged>('chat:hype-train', (e) => handler(e.payload)),
};
