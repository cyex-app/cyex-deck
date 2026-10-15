// V1051-01 The app's own sounds play in Rust; Settings → Sound.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { OutputDevice } from './bindings/OutputDevice';
import type { SoundDecode } from './bindings/SoundDecode';

export const soundIpc = {
  /** Output devices that are on, with Windows' names. */
  devices: () => invoke<OutputDevice[]>('sound_devices'),
  /** A sound file from your settings, or the chime (null). */
  play: (path: string | null) => invoke<void>('sound_play', { path }),
  /** The answer to `onDecode`: interleaved stereo at 48 kHz, or null when it wouldn't decode. */
  decoded: (id: number, pcm: Float32Array | null) =>
    invoke<void>('sound_decoded', pcm ? new Uint8Array(pcm.buffer, pcm.byteOffset, pcm.byteLength) : new Uint8Array(), {
      headers: { 'x-id': String(id) },
    }),
  /** Rust needs a sound file decoded. Main window only. */
  onDecode: (cb: (d: SoundDecode) => void): Promise<UnlistenFn> =>
    listen<SoundDecode>('sound:decode', (e) => cb(e.payload)),
  /** The streams' speaker can be picked now (Settings → Sound). Every window. */
  onSinkReady: (cb: () => void): Promise<UnlistenFn> => listen('sound:sink-ready', () => cb()),
};
