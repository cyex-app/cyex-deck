// V1051-01 The app's sounds play in Rust, so Windows' volume mixer lists them as the app.
// The page only decodes a sound file the first time Rust needs it: Chromium reads every
// type the app takes (Windows can't read ogg and opus). An offline context opens no output.

import { mentionsIpc } from '../ipc/mentions';
import { soundIpc } from '../ipc/sound';
import type { SoundDecode } from '../ipc/bindings/SoundDecode';
import { interleave } from './pcm';

const RATE = 48_000;
/** The longest part of a sound file that plays (sound::MAX_SECONDS). */
const MAX_SECONDS = 30;

export async function decodeSound({ id, path }: SoundDecode): Promise<void> {
  try {
    const bytes = await mentionsIpc.sound(path);
    const buf = await new OfflineAudioContext(2, 1, RATE).decodeAudioData(bytes);
    await soundIpc.decoded(id, interleave(buf, MAX_SECONDS * RATE));
  } catch {
    await soundIpc.decoded(id, null).catch(() => {});
  }
}
