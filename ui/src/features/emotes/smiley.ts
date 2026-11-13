// V110-N18 Twitch's smileys (:) :/ <3 and the robot, monkey and purple sets) come as small
// pictures (about 18 px tall at 1.0 against 28 for an emote). Drawn at emote height they look
// blown up, so chat draws them a bit smaller (user, 2026/11/13). Told apart by the picture's size.

/** Twitch emote ids seen to be small this session, so a line drawn again starts at the right size. */
export const smileys = new Set<string>();

/** Pixel height of a full-size Twitch emote at each CDN scale (`…/1.0`, `/2.0`, `/3.0`). */
const FULL: Record<string, number> = { '1': 28, '2': 56, '3': 112 };

/** A Twitch picture clearly shorter than a full emote at its scale is a smiley. */
export function isSmiley(src: string, naturalHeight: number): boolean {
  const m = /static-cdn\.jtvnw\.net\/emoticons\/v2\/[^/]+\/[^/]+\/[^/]+\/([123])\.0$/.exec(src);
  if (!m || naturalHeight <= 0) return false;
  return naturalHeight < FULL[m[1]] * 0.8;
}
