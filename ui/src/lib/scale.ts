// V105-08 Picture scales: one file per emote, cheer and badge, picked from the screen's density.

/** The sizes Twitch and the emote providers serve. */
export type Scale = 1 | 2 | 4;

/** The scale a 1x/2x/4x srcset would pick for a screen density. */
export function pictureScale(dpr: number): Scale {
  return dpr <= 1 ? 1 : dpr <= 2 ? 2 : 4;
}
