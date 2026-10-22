// Phase 8 chat calls: search (F1, C29), feeds (F4), translation (C17), recordings (X6),
// polls and predictions (C4), power-ups (1.42), GIFs (1.43).

import { Channel, invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { Celebration } from './bindings/Celebration';
import type { FeedEvent } from './bindings/FeedEvent';
import type { FeedLine } from './bindings/FeedLine';
import type { FeedOpened } from './bindings/FeedOpened';
import type { Gif } from './bindings/Gif';
import type { GifInfo } from './bindings/GifInfo';
import type { GifSent } from './bindings/GifSent';
import type { PowerUpBuy } from './bindings/PowerUpBuy';
import type { PowerUps } from './bindings/PowerUps';
import type { Recording } from './bindings/Recording';
import type { RecordingMeta } from './bindings/RecordingMeta';
import type { VotesUpdate } from './bindings/VotesUpdate';

export const chatxIpc = {
  /** `view` null searches every open chat. */
  search: (view: number | null, query: string) => invoke<FeedLine[]>('chat_search', { view, query }),
  feedOpen: (name: string, onEvents: (events: FeedEvent[]) => void) => {
    const channel = new Channel<FeedEvent[]>();
    channel.onmessage = onEvents;
    return invoke<FeedOpened>('feed_open', { name, onEvent: channel });
  },
  /** V107-MF Clear: only the lines the tab hid (`channel/id` keys). */
  feedClear: (name: string, keys: string[]) => invoke<void>('feed_clear', { name, keys }),
  feedClose: (view: number) => invoke<void>('feed_close', { view }),
  translate: (view: number, id: string) => invoke<void>('chat_translate', { view, id }),
  setLanguage: (lang: string) => invoke<void>('chat_set_language', { lang }),
  record: (view: number, on: boolean) => invoke<boolean>('chat_record', { view, on }),
  recording: (view: number) => invoke<boolean>('chat_recording_state', { view }),
  recordings: () => invoke<RecordingMeta[]>('recordings_list'),
  loadRecording: (id: string) => invoke<Recording>('recording_load', { id }),
  deleteRecording: (id: string) => invoke<void>('recording_delete', { id }),
  onRecording: (cb: (e: { channel: string; recording: boolean }) => void) =>
    listen<{ channel: string; recording: boolean }>('chat:recording', (e) => cb(e.payload)),
  votes: (login: string) => invoke<VotesUpdate | null>('chat_votes', { login }),
  onVotes: (cb: (v: VotesUpdate) => void) => listen<VotesUpdate>('chat:votes', (e) => cb(e.payload)),
  vote: (view: number, choice: string, points?: number) => invoke<void>('chat_vote', { view, choice, points }),
  predict: (view: number, outcome: string, points: number) => invoke<void>('chat_predict', { view, outcome, points }),
  powerUps: (view: number) => invoke<PowerUps>('chat_power_ups', { view }),
  buyPowerUp: (view: number, what: PowerUpBuy, bits: number) => invoke<void>('chat_power_up', { view, what, bits }),
  onCelebration: (cb: (c: Celebration) => void) => listen<Celebration>('player:celebration', (e) => cb(e.payload)),
  gifInfo: (view: number) => invoke<GifInfo>('chat_gif_info', { view }),
  gifSearch: (view: number, query: string, offset: number) => invoke<Gif[]>('chat_gif_search', { view, query, offset }),
  gifSend: (view: number, gif: Gif, search: string) =>
    invoke<GifSent>('chat_gif_send', { view, id: gif.id, url: gif.url, search }),
};
