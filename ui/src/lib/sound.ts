// U6 Mention sound: a short two-note chime made with Web Audio, so no sound file ships.
// A mention can play its own file instead (V104-01). Streamer mode can mute both (U4).

import { mentionsIpc } from '../ipc/mentions';
import { streamer } from './streamer.svelte';

let ctx: AudioContext | undefined;

/** The window's one AudioContext: mention sounds and the volume boost (V105-10). */
export function audio(): AudioContext {
  ctx ??= new AudioContext();
  return ctx;
}

export function mentionChime(): void {
  if (streamer.hides('muteSounds')) return;
  const ctx = audio();
  const t = ctx.currentTime;
  const osc = ctx.createOscillator();
  const gain = ctx.createGain();
  osc.type = 'sine';
  osc.frequency.setValueAtTime(880, t);
  osc.frequency.setValueAtTime(1320, t + 0.09);
  gain.gain.setValueAtTime(0.0001, t);
  gain.gain.exponentialRampToValueAtTime(0.12, t + 0.01);
  gain.gain.exponentialRampToValueAtTime(0.0001, t + 0.3);
  osc.connect(gain).connect(ctx.destination);
  osc.start(t);
  osc.stop(t + 0.32);
}

/** Decoded sound files by path, a few at most. */
const files = new Map<string, Promise<AudioBuffer>>();
const MAX_FILES = 8;

/** V104-01 A mention's own sound file, or the chime when it has none or it won't play. */
export async function mentionSound(path?: string): Promise<void> {
  if (!path) return mentionChime();
  if (streamer.hides('muteSounds')) return;
  const audioCtx = audio();
  try {
    let buffer = files.get(path);
    if (!buffer) {
      buffer = mentionsIpc.sound(path).then((bytes) => audioCtx.decodeAudioData(bytes));
      if (files.size >= MAX_FILES) files.delete(files.keys().next().value as string);
      files.set(path, buffer);
    }
    const src = audioCtx.createBufferSource();
    src.buffer = await buffer;
    src.connect(audioCtx.destination);
    src.start();
  } catch {
    files.delete(path);
    mentionChime();
  }
}
