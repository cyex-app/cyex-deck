import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { About } from './bindings/About';
import type { AdState } from './bindings/AdState';
import type { ChatMessage } from './bindings/ChatMessage';
import type { Clip } from './bindings/Clip';
import type { ClipPeriod } from './bindings/ClipPeriod';
import type { ClipPlayback } from './bindings/ClipPlayback';
import type { ClipSort } from './bindings/ClipSort';
import type { MadeClip } from './bindings/MadeClip';
import type { PlayerSession } from './bindings/PlayerSession';
import type { Story } from './bindings/Story';
import type { Storyboard } from './bindings/Storyboard';
import type { StoryRing } from './bindings/StoryRing';
import type { Video } from './bindings/Video';
import type { VideoKind } from './bindings/VideoKind';
import type { VideoSort } from './bindings/VideoSort';

export const playerIpc = {
  live: (login: string, account: string | null) => invoke<PlayerSession>('player_live', { login, account }),
  vod: (id: string, account: string | null) => invoke<PlayerSession>('player_vod', { id, account }),
  close: (id: string) => invoke<void>('player_close', { id }),
  clip: (slug: string) => invoke<ClipPlayback | null>('player_clip', { slug }),
  video: (id: string) => invoke<Video | null>('player_video', { id }),
  storyboard: (id: string) => invoke<Storyboard | null>('player_storyboard', { id }),
  comments: (videoId: string, offset: number) => invoke<ChatMessage[]>('player_comments', { videoId, offset }),
  position: (id: string) => invoke<number | null>('player_position', { id }),
  savePosition: (id: string, at: number, length: number) => invoke<void>('player_save_position', { id, at, length }),
  /** V103-09 With Sign in with Twitch, Twitch's editor opens in the account's Twitch window. */
  createClip: (account: string, channelId: string) => invoke<MadeClip>('player_create_clip', { account, channelId }),
  proxy: () => invoke<string | null>('player_proxy'),
  setProxy: (url: string | null) => invoke<void>('player_set_proxy', { url }),
  onAds: (handler: (id: string, state: AdState) => void) =>
    listen<{ id: string; state: AdState }>('player:ads', (e) => handler(e.payload.id, e.payload.state)),
};

export const channelIpc = {
  about: (login: string) => invoke<About | null>('channel_about', { login }),
  clips: (login: string, period: ClipPeriod, sort: ClipSort) => invoke<Clip[]>('channel_clips', { login, period, sort }),
  videos: (login: string, kind: VideoKind | null, sort: VideoSort) =>
    invoke<Video[]>('channel_videos', { login, kind, sort }),
  storyRing: (channelId: string, account: string | null) =>
    invoke<StoryRing | null>('channel_story_ring', { channelId, account }),
  stories: (channelId: string, account: string) => invoke<Story[]>('channel_stories', { channelId, account }),
  storiesSeen: (channelId: string, account: string, createdAt: string) =>
    invoke<void>('channel_stories_seen', { channelId, account, createdAt }),
};
