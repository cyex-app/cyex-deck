// V108-N01 Clipping a VOD at the current time.

/** Where a clip of the video ends: the play head, whole seconds, inside the video. A clip
 *  covers the seconds before, so the very start has nothing to clip. */
export function vodClipOffset(time: number, length: number): number {
  const t = Number.isFinite(time) ? Math.floor(time) : 0;
  return Math.max(0, length > 0 ? Math.min(t, Math.floor(length)) : t);
}

/** Only a video of this tab's own channel can be clipped here (Twitch clips as the channel). */
export function ownVideo(videoChannelLogin: string | undefined, tabLogin: string | undefined): boolean {
  return !!videoChannelLogin && !!tabLogin && videoChannelLogin.toLowerCase() === tabLogin.toLowerCase();
}
