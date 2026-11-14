// V111-13 The usercard's facts as text: last live, the gifter, the chat colour, roles in other
// channels, and which lookups the card makes for what Settings > Usercard shows.

import type { IvrUser } from '../../ipc/bindings/IvrUser';
import type { OtherRole } from '../../ipc/bindings/OtherRole';
import type { RoleCounts } from '../../ipc/bindings/RoleCounts';
import type { SubStatus } from '../../ipc/bindings/SubStatus';
import type { UsercardShow } from '../../ipc/bindings/UsercardShow';
import { formatDate, formatNumber, formatTime } from '../../lib/format.ts';

/** What the card asks for; an item that is off asks nobody. The chat colour comes from the Twitch profile. */
export function lookups(show: UsercardShow) {
  return {
    ivr: show.lastLive || show.verifiedBot,
    sub: show.sub,
    followAge: show.followAge,
    names: show.names,
    roles: show.roles,
    otherMods: show.otherModCounts,
    seventv: show.seventvProfile,
  };
}

/** "Last live 2026/11/02, 19:40", or "Live since 19:40" while live. Null when unknown. */
export function lastLiveText(ivr: IvrUser | null | undefined): string | null {
  if (!ivr?.lastLive) return null;
  const d = new Date(ivr.lastLive);
  if (Number.isNaN(d.getTime())) return null;
  return ivr.live ? `Live since ${formatTime(d)}` : `Last live ${formatDate(d)}, ${formatTime(d)}`;
}

/** V111-18 Under the title in the hover tip: the time is when that stream started, not ended. */
export function lastLiveHint(ivr: IvrUser | null | undefined): string {
  if (!ivr?.lastLive || ivr.live) return '';
  const d = new Date(ivr.lastLive);
  return Number.isNaN(d.getTime()) ? '' : `Stream started ${formatDate(d)}, ${formatTime(d)}`;
}

/** "Gifted by Name", "Gifted anonymously", or "gifted" alone when Twitch doesn't say. */
export function giftText(sub: Pick<SubStatus, 'gift' | 'gifter' | 'giftAnonymous'>, showGifter: boolean): string {
  if (!sub.gift) return '';
  if (!showGifter) return 'gifted';
  if (sub.gifter) return `gifted by ${sub.gifter}`;
  return sub.giftAnonymous ? 'gifted anonymously' : 'gifted';
}

/** Their colour as `#RRGGBB`: the Twitch profile's, else api.ivr.fi's. Null when they never set one. */
export function chatHex(profileColor: string | null | undefined, ivr: IvrUser | null | undefined): string | null {
  const ok = (c: string | null | undefined) => (c && /^#[0-9a-f]{6}$/i.test(c) ? c.toUpperCase() : null);
  return ok(profileColor) ?? ok(ivr?.chatColor);
}

export const ROLE_LABELS: Record<OtherRole, string> = { mod: 'Mod', vip: 'VIP', founder: 'Founder', artist: 'Artist' };

/** The roles they hold anywhere, with counts, in card order; none held gives []. */
export function roleSummary(c: RoleCounts): { role: OtherRole; count: number; text: string }[] {
  const all: [OtherRole, number][] = [
    ['mod', c.mods],
    ['vip', c.vips],
    ['founder', c.founders],
    ['artist', c.artists],
  ];
  return all
    .filter(([, n]) => n > 0)
    .map(([role, n]) => ({ role, count: n, text: `${ROLE_LABELS[role]} in ${formatNumber(n)}` }));
}

/** "since 2020/08/11", or "" when roles.tv has no date. */
export function sinceText(rfc: string | null): string {
  if (!rfc) return '';
  const d = new Date(rfc);
  return Number.isNaN(d.getTime()) ? '' : `since ${formatDate(d)}`;
}
