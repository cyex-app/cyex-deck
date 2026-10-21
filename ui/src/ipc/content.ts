// V107-R39 The stream manager's Clips and Videos panels: your channel's clips and videos.

import { invoke } from '@tauri-apps/api/core';

import type { ClipFilter } from './bindings/ClipFilter';
import type { ClipPage } from './bindings/ClipPage';
import type { ContentDeleted } from './bindings/ContentDeleted';
import type { VideoKind } from './bindings/VideoKind';
import type { VideoPage } from './bindings/VideoPage';

export const contentIpc = {
  clips: (filter: ClipFilter) => invoke<ClipPage>('content_clips', { filter }),
  deleteClips: (slugs: string[]) => invoke<ContentDeleted>('content_delete_clips', { slugs }),
  videos: (kind: VideoKind | null, cursor: string | null) => invoke<VideoPage>('content_videos', { kind, cursor }),
  /** The title Twitch kept. */
  renameVideo: (id: string, title: string) => invoke<string>('content_rename_video', { id, title }),
  /** Whether it's public now. */
  publishVideo: (id: string, isPublic: boolean) => invoke<boolean>('content_publish_video', { id, public: isPublic }),
  /** Several at once; `gone` lists the ones changed. */
  publishVideos: (ids: string[], isPublic: boolean) => invoke<ContentDeleted>('content_publish_videos', { ids, public: isPublic }),
  deleteVideos: (ids: string[]) => invoke<ContentDeleted>('content_delete_videos', { ids }),
};
