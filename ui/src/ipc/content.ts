// V107-R39 The stream manager's Clips and Videos panels: your channel's clips and videos.

import { invoke } from '@tauri-apps/api/core';

import type { ClipFilter } from './bindings/ClipFilter';
import type { ClipPage } from './bindings/ClipPage';
import type { SmBulk } from './bindings/SmBulk';
import type { VideoKind } from './bindings/VideoKind';
import type { VideoPage } from './bindings/VideoPage';

/** V109-C04 `account`: whose channel (the creator dashboard's pick); unset, the stream manager's. */
export const contentIpc = {
  clips: (filter: ClipFilter) => invoke<ClipPage>('content_clips', { filter }),
  deleteClips: (slugs: string[]) => invoke<SmBulk>('content_delete_clips', { slugs }),
  videos: (kind: VideoKind | null, cursor: string | null, account?: string | null) => invoke<VideoPage>('content_videos', { kind, cursor, account }),
  /** The title Twitch kept. */
  renameVideo: (id: string, title: string, account?: string | null) => invoke<string>('content_rename_video', { id, title, account }),
  /** Whether it's public now. */
  publishVideo: (id: string, isPublic: boolean, account?: string | null) =>
    invoke<boolean>('content_publish_video', { id, public: isPublic, account }),
  /** Several at once, paced by Rust; `done` lists the ones changed. */
  publishVideos: (ids: string[], isPublic: boolean, account?: string | null) =>
    invoke<SmBulk>('content_publish_videos', { ids, public: isPublic, account }),
  deleteVideos: (ids: string[], account?: string | null) => invoke<SmBulk>('content_delete_videos', { ids, account }),
};
