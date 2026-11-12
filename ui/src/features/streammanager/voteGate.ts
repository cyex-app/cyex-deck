// V110-N13 Polls and predictions need Affiliate or Partner (predictions run on channel
// points, which come with it). Twitch turns both down on any other channel without a
// reason, so the cards say so up front. Unknown (the read failed) keeps the form.

import type { Profile } from '../../ipc/bindings/Profile';

export type VoteKind = 'poll' | 'prediction';

/** `true` Affiliate or Partner, `false` neither, `null` unknown. */
export function votesAllowed(p: Pick<Profile, 'affiliate' | 'partner'> | null | undefined): boolean | null {
  if (!p) return null;
  return p.affiliate || p.partner;
}

/** What the card shows instead of the form on a channel without Affiliate. */
export const NOT_ALLOWED: Record<VoteKind, string> = {
  poll: 'Polls come with Affiliate.',
  prediction: 'Predictions need channel points. They come with Affiliate.',
};

/** The field to focus when the form opens: the first empty one, else the first. */
export function firstEmpty(values: readonly string[]): number {
  const i = values.findIndex((v) => !v.trim());
  return i < 0 ? 0 : i;
}
