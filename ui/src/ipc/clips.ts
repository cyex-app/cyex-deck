// V109-C03 The clips manager: an account's own clips and its channel's clips.

import { invoke } from '@tauri-apps/api/core';

import type { ClipFilter } from './bindings/ClipFilter';
import type { ClipPage } from './bindings/ClipPage';
import type { SmBulk } from './bindings/SmBulk';

export const clipsIpc = {
  list: (account: string, filter: ClipFilter) => invoke<ClipPage>('clips_list', { account, filter }),
  /** Paced by Rust, 25 a request; `done` lists the ones Twitch deleted. Stop: `smIpc.bulkStop`. */
  remove: (account: string, slugs: string[]) => invoke<SmBulk>('clips_delete', { account, slugs }),
  /** V109-C05 Twitch’s title-only edit; the title Twitch kept, or its refusal as the error. */
  rename: (account: string, slug: string, title: string) => invoke<string>('clips_rename', { account, slug, title }),
  /** Twitch's clip editor in the account's Twitch window; false in a mock run. */
  edit: (account: string, slug: string) => invoke<boolean>('clips_edit', { account, slug }),
};
