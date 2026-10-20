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
};

/** The `ImageError` a command rejected with, or null for anything else. */
export function imageError(e: unknown): ImageError | null {
  const o = e as Partial<ImageError> | null;
  return o && typeof o === 'object' && typeof o.kind === 'string' && typeof o.message === 'string'
    ? (o as ImageError)
    : null;
}
