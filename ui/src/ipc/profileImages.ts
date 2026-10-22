// V107-R19 Profile picture, banner and offline image.

import { invoke } from '@tauri-apps/api/core';

import type { ImageError } from './bindings/ImageError';
import type { ProfileImageKind } from './bindings/ProfileImageKind';
import type { ProfileImagesView } from './bindings/ProfileImagesView';

export const profileImagesIpc = {
  get: (account: string) => invoke<ProfileImagesView>('profile_images_get', { account }),
  change: (account: string, kind: ProfileImageKind, bytes: Uint8Array) =>
    invoke<ProfileImagesView>('profile_images_change', bytes, { headers: { 'x-account': account, 'x-kind': kind } }),
  restore: (account: string, kind: ProfileImageKind) =>
    invoke<ProfileImagesView>('profile_images_restore', { account, kind }),
  mocked: () => invoke<boolean>('profile_images_mocked'),
  /** V107-R19c Puts an image from the history up again. */
  use: (account: string, kind: ProfileImageKind, id: string) =>
    invoke<ProfileImagesView>('profile_images_use', { account, kind, id }),
  /** V107-R19c Drops one image from the history. */
  forget: (account: string, kind: ProfileImageKind, id: string) =>
    invoke<void>('profile_images_forget', { account, kind, id }),
  /** V107-Mh Back to Twitch's default (avatar, offline image). */
  remove: (account: string, kind: ProfileImageKind) =>
    invoke<ProfileImagesView>('profile_images_remove', { account, kind }),
};

/** The `ImageError` a command rejected with, or null for anything else. */
export function imageError(e: unknown): ImageError | null {
  const o = e as Partial<ImageError> | null;
  return o && typeof o === 'object' && typeof o.kind === 'string' && typeof o.message === 'string'
    ? (o as ImageError)
    : null;
}
