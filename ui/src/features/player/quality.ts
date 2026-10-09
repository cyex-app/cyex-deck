// Which hls.js level to play (1.1). "Auto" fits the player on screen: the smallest
// quality at least as tall as the player, so small tiles don't fetch 1080p. hls.js's own
// bandwidth guess is off: low latency segments arrive at the speed they're made.

import type { Quality } from '../../ipc/bindings/Quality';

export interface LevelLike {
  height: number;
  frameRate?: number;
  bitrate: number;
}

/** A player's choice: the setting's qualities, or audio only (P1). */
export type Choice = Quality | 'audio';

/** Index into `levels` (any order) for `choice`, -1 when there's no video level. */
export function pickLevel(levels: LevelLike[], choice: Quality, playerPx: number): number {
  const best = levels
    .map((l, i) => ({ l, i }))
    .filter(({ l }) => l.height > 0)
    .sort((a, b) => b.l.height - a.l.height || (b.l.frameRate ?? 0) - (a.l.frameRate ?? 0) || b.l.bitrate - a.l.bitrate);
  if (!best.length) return -1;
  if (choice === 'source') return best[0].i;
  if (choice === 'auto') {
    const fits = best.filter(({ l }) => l.height >= playerPx);
    if (!fits.length) return best[0].i;
    const height = Math.min(...fits.map(({ l }) => l.height));
    return fits.find(({ l }) => l.height === height)!.i;
  }
  const max = Number(choice);
  return (best.find(({ l }) => l.height <= max) ?? best[best.length - 1]).i;
}

/** "1080p60", "720p", like Twitch names them. */
export function levelName(l: LevelLike): string {
  const fps = Math.round(l.frameRate ?? 0);
  return `${l.height}p${fps > 30 ? fps : ''}`;
}
