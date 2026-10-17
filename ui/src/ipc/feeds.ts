// V106-07 Feed tabs of their own kind: live notifications, AutoMod, whispers.

import { Channel, invoke } from '@tauri-apps/api/core';

import type { FeedKind } from './bindings/FeedKind';
import type { FeedsOpened } from './bindings/FeedsOpened';
import type { KindEvent } from './bindings/KindEvent';

export const feedsIpc = {
  open: (kind: FeedKind, on: (e: KindEvent[]) => void) => {
    const channel = new Channel<KindEvent[]>();
    channel.onmessage = on;
    return invoke<FeedsOpened>('feeds_open', { kind, onEvent: channel });
  },
  close: (view: number) => invoke<void>('feeds_close', { view }),
  clear: (kind: FeedKind) => invoke<void>('feeds_clear', { kind }),
};
