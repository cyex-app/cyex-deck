// Image URLs for emotes and cheermotes. Providers and ids come from Rust (1.8, 1.40).

import type { Cheermote } from '../../ipc/bindings/Cheermote';
import type { CheerTier } from '../../ipc/bindings/CheerTier';
import type { Provider } from '../../ipc/bindings/Provider';
import type { Theme } from '../../ipc/bindings/Theme';

export interface Img {
  src: string;
  srcset: string;
}

export interface Frames {
  /** The provider has a moving version (7TV, FFZ need to know). */
  animated?: boolean;
  /** E2 Show the still frame. */
  still?: boolean;
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
  return {
    src: emoteUrl(provider, id, theme, 1, f),
    srcset: `${emoteUrl(provider, id, theme, 2, f)} 2x, ${emoteUrl(provider, id, theme, 4, f)} 4x`,
  };
}

/** The biggest picture, for hover previews (E6) and emote cards (1.41). */
export function emoteLarge(provider: Provider, id: string, theme: Theme, f: Frames = {}): string {
  return emoteUrl(provider, id, theme, 4, f);
}

/** The tier a cheer of `bits` shows as: the highest one it reaches. */
export function cheerTier(c: Cheermote, bits: number): CheerTier | undefined {
  let best: CheerTier | undefined;
  for (const t of c.tiers) if (t.bits <= bits && (!best || t.bits > best.bits)) best = t;
  return best ?? c.tiers[0];
}

export function cheerImg(c: Cheermote, tier: CheerTier, theme: Theme): Img {
  const url = (scale: string) =>
    c.template
      .replace('PREFIX', c.prefix.toLowerCase())
      .replace('BACKGROUND', theme)
      .replace('ANIMATION', 'animated')
      .replace('TIER', String(tier.bits))
      .replace('SCALE', scale)
      .replace('EXTENSION', 'gif');
  return { src: url('1'), srcset: `${url('2')} 2x, ${url('4')} 4x` };
}
