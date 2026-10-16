// V1051-01 The app's own sounds play in Rust; Settings → Sound.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { SoundDevices } from './bindings/SoundDevices';
import type { SoundDecode } from './bindings/SoundDecode';

export const soundIpc = {
  /** Output devices that are on, with Windows' names, and the default one's name. */
  devices: () => invoke<SoundDevices>('sound_devices'),
  /** Lets the page use the microphone (`on`) or takes it back; only around picking a speaker. */
  permit: (on: boolean) => invoke<void>('sound_permit', { on }),
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
};
