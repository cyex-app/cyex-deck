// V111-44 How tall a chat emote draws, as a share of a full emote (2.15em × the emote size
// setting, design/emotes.css `--eh`). The rule is 7TV's: a 1x picture is 32 px tall and at most
// 96 wide, so an emote wider than 3:1 draws shorter instead of wider. A picture clearly smaller
// than its provider's full 1x height (a tiny emote like ppL, 9×7) draws at its own size; CSS keeps
// it at least the text's x-height. Twitch emotes keep the smiley rule (smiley.ts).

import type { Provider } from '../../ipc/bindings/Provider';

/** Widest an emote draws, in full emote heights (7TV's 96 / 32). */
export const WIDE_MAX = 3;

/** A full emote's 1x height per provider. */
const FULL_1X: Partial<Record<Provider, number>> = { seventv: 32, bttv: 28, ffz: 32 };

/** Below this share of a full 1x height a picture counts as small (as Twitch smileys, smiley.ts). */
const SMALL = 0.8;

/** The share a provider's 1x size gives: shorter for wider than 3:1, own size for a small one. */
export function sizeShare(provider: Provider, size: readonly [number, number] | null | undefined): number {
  if (!size || size[0] <= 0 || size[1] <= 0) return 1;
  const wide = Math.min(1, (WIDE_MAX * size[1]) / size[0]);
  return Math.min(wide, smallShare(provider, size[1]) ?? 1);
}

/** The share of a picture `h1x` tall at 1x, when it is small for its provider; else `null`. */
export function smallShare(provider: Provider, h1x: number): number | null {
  const full = FULL_1X[provider];
  if (!full || h1x <= 0 || h1x >= full * SMALL) return null;
  return h1x / full;
}

/** The 1x height of a loaded picture, from its CDN URL's scale; `null` for unknown URLs (saved copies). */
export function height1x(provider: Provider, src: string, naturalHeight: number): number | null {
  if (naturalHeight <= 0) return null;
  const re =
    provider === 'seventv'
      ? /cdn\.7tv\.app\/emote\/[^/]+\/([1-4])x(?:_static)?\.webp$/
      : provider === 'bttv'
        ? /cdn\.betterttv\.net\/emote\/[^/]+\/(?:static\/)?([1-3])x\.webp$/
        : provider === 'ffz'
          ? /cdn\.frankerfacez\.com\/emote\/[^/]+\/(?:animated\/)?([1-4])$/
          : null;
  const m = re?.exec(src);
  return m ? naturalHeight / Number(m[1]) : null;
}

/** Small emotes seen this session by `provider:id`, with their share, so a line drawn again starts
 *  at the right size (the provider's size can't tell: 7TV's is from the aspect ratio). */
export const smallSeen = new Map<string, number>();
