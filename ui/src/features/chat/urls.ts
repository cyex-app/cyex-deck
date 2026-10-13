// Image URLs for emotes and cheermotes. Providers and ids come from Rust (1.8, 1.40).
// One URL per picture (V105-08): a srcset lets Chromium swap in a bigger copy it has cached
// (after a hover preview), and that copy animates on its own clock.

import type { Cheermote } from '../../ipc/bindings/Cheermote';
import type { CheerTier } from '../../ipc/bindings/CheerTier';
import type { Provider } from '../../ipc/bindings/Provider';
import type { Theme } from '../../ipc/bindings/Theme';
import { saved } from './saved.ts';

export interface Img {
  src: string;
}

export interface Frames {
  /** The provider has a moving version (7TV, FFZ need to know). */
  animated?: boolean;
  /** E2 Show the still frame. */
  still?: boolean;
  /** Picture scale, from `density`; 1 when left out. */
  scale?: 1 | 2 | 4;
}

/** V105-08 Picture scale for a screen density: what a 1x/2x/4x srcset would pick. */
export function pictureScale(dpr: number): 1 | 2 | 4 {
  return dpr <= 1 ? 1 : dpr <= 2 ? 2 : 4;
}

/** V105-08 One badge picture for the scale. */
export function badgeSrc(b: { image1x: string; image2x: string; image4x: string }, scale: 1 | 2 | 4): string {
  return scale === 1 ? b.image1x : scale === 2 ? b.image2x : b.image4x;
}

/** Picture URL at a scale: 1, 2 or 4 (3 for providers that stop there). */
function emoteUrl(provider: Provider, id: string, theme: Theme, scale: 1 | 2 | 4, f: Frames): string {
  const e = encodeURIComponent(id);
  switch (provider) {
    case 'twitch':
      // Twitch emotes come in dark and light variants; `static` is the still frame.
      return `https://static-cdn.jtvnw.net/emoticons/v2/${e}/${f.still ? 'static' : 'default'}/${theme}/${scale === 4 ? 3 : scale}.0`;
    case 'seventv':
      return `https://cdn.7tv.app/emote/${e}/${scale}x${f.animated && f.still ? '_static' : ''}.webp`;
    case 'bttv':
      return `https://cdn.betterttv.net/emote/${e}/${f.animated && f.still ? 'static/' : ''}${scale === 4 ? 3 : scale}x.webp`;
    case 'ffz':
      return `https://cdn.frankerfacez.com/emote/${e}/${f.animated && !f.still ? 'animated/' : ''}${scale}`;
  }
}

export function emoteImg(provider: Provider, id: string, theme: Theme, f: Frames = {}): Img {
  const copy = saved(`emote:${provider}:${id}`);
  if (copy) return { src: copy };
  return { src: emoteUrl(provider, id, theme, f.scale ?? 1, f) };
}

/** The biggest picture, for hover previews (E6) and emote cards (1.41). */
export function emoteLarge(provider: Provider, id: string, theme: Theme, f: Frames = {}): string {
  return saved(`emote:${provider}:${id}`) ?? emoteUrl(provider, id, theme, 4, f);
}

/** The tier a cheer of `bits` shows as: the highest one it reaches. */
export function cheerTier(c: Cheermote, bits: number): CheerTier | undefined {
  let best: CheerTier | undefined;
  for (const t of c.tiers) if (t.bits <= bits && (!best || t.bits > best.bits)) best = t;
  return best ?? c.tiers[0];
}

export function cheerImg(c: Cheermote, tier: CheerTier, theme: Theme, scale: 1 | 2 | 4 = 1): Img {
  const url = (scale: string) =>
    c.template
      .replace('PREFIX', c.prefix.toLowerCase())
      .replace('BACKGROUND', theme)
      .replace('ANIMATION', 'animated')
      .replace('TIER', String(tier.bits))
      .replace('SCALE', scale)
      .replace('EXTENSION', 'gif');
  return { src: url(String(scale)) };
}
