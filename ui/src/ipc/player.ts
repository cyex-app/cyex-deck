import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { About } from './bindings/About';
import type { AdState } from './bindings/AdState';
import type { ChatMessage } from './bindings/ChatMessage';
import type { Clip } from './bindings/Clip';
import type { ClipPeriod } from './bindings/ClipPeriod';
import type { ClipPlayback } from './bindings/ClipPlayback';
import type { ClipSaveProgress } from './bindings/ClipSaveProgress';
import type { ClipSort } from './bindings/ClipSort';
import type { MadeClip } from './bindings/MadeClip';
import type { PlayerSession } from './bindings/PlayerSession';
import type { ReplayRange } from './bindings/ReplayRange';
import type { ReplayStart } from './bindings/ReplayStart';
import type { Story } from './bindings/Story';
import type { Storyboard } from './bindings/Storyboard';
import type { StoryRing } from './bindings/StoryRing';
import type { Video } from './bindings/Video';
import type { VideoKind } from './bindings/VideoKind';
import type { VideoSort } from './bindings/VideoSort';
import type { VodProgress } from './bindings/VodProgress';

export const playerIpc = {
  /** `replay`: the player's key, so what it plays is kept for going back (V108-N24). */
  live: (login: string, account: string | null, replay: string | null = null) =>
    invoke<PlayerSession>('player_live', { login, account, replay }),
  /** V108-N24 Live session `id`'s replay from `at` (ms), null when it holds nothing. */
  replay: (id: string, at: number, audio: boolean) => invoke<ReplayStart | null>('player_replay', { id, at, audio }),
  replayRange: (id: string, audio: boolean) => invoke<ReplayRange | null>('player_replay_range', { id, audio }),
  replayGone: (id: string, uri: string) => invoke<void>('player_replay_gone', { id, uri }),
  vod: (id: string, account: string | null) => invoke<PlayerSession>('player_vod', { id, account }),
  /** V1061-03 The VOD Twitch lets this account rewind in, or null. */
  rewind: (login: string, account: string | null) => invoke<string | null>('player_rewind', { login, account }),
  close: (id: string) => invoke<void>('player_close', { id }),
  /** V103-07 Another player plays this channel now, in any window. */
  playing: (login: string) => invoke<boolean>('player_playing', { login }),
  clip: (slug: string) => invoke<ClipPlayback | null>('player_clip', { slug }),
  video: (id: string) => invoke<Video | null>('player_video', { id }),
  storyboard: (id: string) => invoke<Storyboard | null>('player_storyboard', { id }),
  comments: (videoId: string, offset: number) => invoke<ChatMessage[]>('player_comments', { videoId, offset }),
  position: (id: string) => invoke<number | null>('player_position', { id }),
  savePosition: (id: string, at: number, length: number) => invoke<void>('player_save_position', { id, at, length }),
  /** V107-Mv How far each VOD was watched; unstarted ones are left out. */
  progress: (ids: string[]) => invoke<VodProgress[]>('player_progress', { ids }),
  /** V103-09 With Sign in with Twitch, Twitch's editor opens in the account's Twitch window. */
  createClip: (account: string, channelId: string) => invoke<MadeClip>('player_create_clip', { account, channelId }),
  /** V108-N25 Clips the live broadcast up to `offsetSecs` into it (GQL sign-in). */
  createClipAt: (account: string, channelId: string, offsetSecs: number) =>
    invoke<MadeClip>('player_create_clip_at', { account, channelId, offsetSecs }),
  /** V108-N01 Clips a VOD of the channel up to `offsetSecs` into it. */
  createClipFromVideo: (account: string, channelId: string, videoId: string, offsetSecs: number) =>
    invoke<MadeClip>('player_create_clip_from_video', { account, channelId, videoId, offsetSecs }),
  /** V108-N02 Saves a clip as MP4 through a save dialog: the path, null when cancelled. */
  saveClip: (slug: string, height: number | null) => invoke<string | null>('player_clip_save', { slug, height }),
  cancelClipSave: (slug: string) => invoke<boolean>('player_clip_save_cancel', { slug }),
  onClipSave: (handler: (p: ClipSaveProgress) => void) =>
    listen<ClipSaveProgress>('player:clip-save', (e) => handler(e.payload)),
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
