// Image URLs for emotes and cheermotes. Providers and ids come from Rust (1.8, 1.40).

import type { Cheermote } from '../../ipc/bindings/Cheermote';
import type { CheerTier } from '../../ipc/bindings/CheerTier';
import type { Provider } from '../../ipc/bindings/Provider';
import type { Theme } from '../../ipc/bindings/Theme';

export interface Img {
  src: string;
  srcset: string;
}

/** Twitch emotes come in dark and light variants. */
export function emoteImg(provider: Provider, id: string, theme: Theme): Img {
  switch (provider) {
    case 'twitch': {
      const base = `https://static-cdn.jtvnw.net/emoticons/v2/${encodeURIComponent(id)}/default/${theme}`;
      return { src: `${base}/1.0`, srcset: `${base}/2.0 2x, ${base}/3.0 4x` };
    }
    case 'seventv':
      return {
        src: `https://cdn.7tv.app/emote/${id}/1x.webp`,
        srcset: `https://cdn.7tv.app/emote/${id}/2x.webp 2x, https://cdn.7tv.app/emote/${id}/4x.webp 4x`,
      };
    case 'bttv':
      return {
        src: `https://cdn.betterttv.net/emote/${id}/1x`,
        srcset: `https://cdn.betterttv.net/emote/${id}/2x 2x, https://cdn.betterttv.net/emote/${id}/3x 4x`,
      };
    case 'ffz':
      return {
        src: `https://cdn.frankerfacez.com/emote/${id}/1`,
        srcset: `https://cdn.frankerfacez.com/emote/${id}/2 2x, https://cdn.frankerfacez.com/emote/${id}/4 4x`,
      };
  }
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
