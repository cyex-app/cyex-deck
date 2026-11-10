// Image URLs for emotes and cheermotes. Providers and ids come from Rust (1.8, 1.40).
// One URL per picture (V105-08): a srcset lets Chromium swap in a bigger copy it has cached
// (after a hover preview), and that copy animates on its own clock.

import type { Cheermote } from '../../ipc/bindings/Cheermote';
import type { CheerTier } from '../../ipc/bindings/CheerTier';
import type { Provider } from '../../ipc/bindings/Provider';
import type { Theme } from '../../ipc/bindings/Theme';
import { type Scale } from '../../lib/scale.ts';
import { saved } from './saved.ts';

export interface Img {
  src: string;
}

/** How a picture moves. */
export interface Motion {
  /** The provider has a moving version (7TV, FFZ need to know). */
  animated?: boolean;
  /** E2 Show the still frame. */
  still?: boolean;
}

export interface Frames extends Motion {
  /** Picture scale, from `density`. */
  scale: Scale;
}

/** V105-08 One badge picture for the scale. */
export function badgeSrc(b: { image1x: string; image2x: string; image4x: string }, scale: Scale): string {
  return scale === 1 ? b.image1x : scale === 2 ? b.image2x : b.image4x;
}

/** V110-B1 A badge's 1x/2x/4x pictures as a srcset: the screen picks the one that matches size
 *  times density (also per screen, and fractional ones like 150 %), and a missing size is skipped. */
export function badgeSet(b: { image1x: string; image2x: string; image4x: string }): string | undefined {
  const parts: string[] = [];
  const seen = new Set<string>();
  for (const [url, d] of [[b.image1x, '1x'], [b.image2x, '2x'], [b.image4x, '4x']] as const) {
    if (!url || seen.has(url)) continue;
    seen.add(url);
    parts.push(`${url} ${d}`);
  }
  // One picture only: no descriptor gain, the plain src does it.
  return parts.length > 1 ? parts.join(', ') : undefined;
}

/** V110-B1 The picture for a badge's hover preview (112 px tall): the largest one there is (the 256 px
 *  one of cyex badges uploaded after that size came, else the 4x). */
export function badgeLarge(b: { image1x: string; image2x: string; image4x: string; imageLarge?: string | null }): string {
  return b.imageLarge || b.image4x || b.image2x || b.image1x;
}

/** Twitch's two `<3` hearts: their 3.0 picture is another drawing (an outlined heart, not the
 *  pixel heart of 1.0 and 2.0), so they stop at 2.0 and look the same on every screen (user). */
const TWITCH_STOP_AT_2 = new Set(['9', '555555584']);

/** Picture URL at a scale: 1, 2 or 4 (3 for providers that stop there). */
function emoteUrl(provider: Provider, id: string, theme: Theme, scale: Scale, f: Motion): string {
  const e = encodeURIComponent(id);
  switch (provider) {
    case 'twitch': {
      // Twitch emotes come in dark and light variants; `static` is the still frame.
      const s = TWITCH_STOP_AT_2.has(id) ? Math.min(scale, 2) : scale === 4 ? 3 : scale;
      return `https://static-cdn.jtvnw.net/emoticons/v2/${e}/${f.still ? 'static' : 'default'}/${theme}/${s}.0`;
    }
    case 'seventv':
      return `https://cdn.7tv.app/emote/${e}/${scale}x${f.animated && f.still ? '_static' : ''}.webp`;
    case 'bttv':
      return `https://cdn.betterttv.net/emote/${e}/${f.animated && f.still ? 'static/' : ''}${scale === 4 ? 3 : scale}x.webp`;
    case 'ffz':
      return `https://cdn.frankerfacez.com/emote/${e}/${f.animated && !f.still ? 'animated/' : ''}${scale}`;
  }
}

export function emoteImg(provider: Provider, id: string, theme: Theme, f: Frames): Img {
  const copy = saved(`emote:${provider}:${id}`);
  if (copy) return { src: copy };
  return { src: emoteUrl(provider, id, theme, f.scale, f) };
}

/** The biggest picture, for hover previews (E6) and emote cards (1.41). */
export function emoteLarge(provider: Provider, id: string, theme: Theme, f: Motion = {}): string {
  return saved(`emote:${provider}:${id}`) ?? emoteUrl(provider, id, theme, 4, f);
}

/** The tier a cheer of `bits` shows as: the highest one it reaches. */
export function cheerTier(c: Cheermote, bits: number): CheerTier | undefined {
  let best: CheerTier | undefined;
  for (const t of c.tiers) if (t.bits <= bits && (!best || t.bits > best.bits)) best = t;
  return best ?? c.tiers[0];
}

export function cheerImg(c: Cheermote, tier: CheerTier, theme: Theme, scale: Scale): Img {
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
