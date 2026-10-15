// V105-11 Image uploads from the chat box.

import { type Channel, invoke } from '@tauri-apps/api/core';

import type { UploadEvent } from './bindings/UploadEvent';
import type { UploadItem } from './bindings/UploadItem';

export const uploadIpc = {
  /** Pasted image data; the id goes into `images`. */
  stage: (bytes: Uint8Array, ext: string) => invoke<string>('upload_stage', bytes, { headers: { 'x-ext': ext } }),
  images: (channel: string, items: UploadItem[], onEvent: Channel<UploadEvent>) =>
    invoke<void>('upload_images', { channel, items, onEvent }),
};
