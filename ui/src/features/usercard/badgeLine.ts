/** V109-F71 The usercard's badge line: earned badges first, no role badges, no 3rd-party sets. */

/** Roles shown in chat anyway (partner and verified are identities and stay). */
const ROLES = new Set(['broadcaster', 'moderator', 'lead_moderator', 'vip', 'staff', 'admin', 'global_mod']);
/** Channel badges earned by watching, paying or playing along. */
const CHANNEL = new Set([
  'subscriber', 'founder', 'sub-gifter', 'sub-gift-leader', 'bits', 'bits-leader', 'hype-train', 'predictions',
]);
/** Who they are, not what they did. */
const IDENTITY = new Set(['partner', 'verified', 'turbo', 'premium', 'artist-badge']);

export const isRole = (set: string): boolean => ROLES.has(set);

/** 0 event and other limited badges, 1 channel-earned, 2 identity. */
function rank(set: string): number {
  if (CHANNEL.has(set)) return 1;
  if (IDENTITY.has(set)) return 2;
  return 0;
}

/** Twitch badges only: roles dropped, then events, channel-earned, identity (stable). */
export function cardBadges<T extends { set: string }>(list: readonly T[]): T[] {
  return list
    .map((b, i) => ({ b, i }))
    .filter(({ b }) => !isRole(b.set))
    .sort((x, y) => rank(x.b.set) - rank(y.b.set) || x.i - y.i)
    .map(({ b }) => b);
}
