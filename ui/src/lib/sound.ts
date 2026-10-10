// U6 Mention sound: a short two-note chime made with Web Audio, so no sound file ships.
// Streamer mode can mute it (U4).

import { streamer } from './streamer.svelte';

let ctx: AudioContext | undefined;

export function mentionChime(): void {
  if (streamer.hides('muteSounds')) return;
  ctx ??= new AudioContext();
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
