// V111-24 What the Add-a-tab preview shows for a suggestion, from data the search already has.
import type { ChannelInfo } from '../../ipc/bindings/ChannelInfo';
import { uptimeShort } from '../browse/lists.ts';
import { formatNumber } from '../../lib/format.ts';

/** Widths Twitch's preview CDN serves cleanly, 16:9. */
const WIDTHS = [192, 320, 440, 640];

/** The smallest stock width that covers `cssWidth` css px at this pixel ratio. */
export function previewWidth(cssWidth: number, dpr: number): number {
  const need = Math.ceil(cssWidth * (dpr > 0 ? dpr : 1));
  return WIDTHS.find((w) => w >= need) ?? WIDTHS[WIDTHS.length - 1];
}

/** Twitch's live thumbnail, sized for the box. Twitch renders a new one about every 5 minutes;
 *  the stamp changes each minute, so a reopened or revisited preview is never staler than that. */
export function previewUrl(login: string, cssWidth: number, dpr: number, now = Date.now()): string {
  const w = previewWidth(cssWidth, dpr);
  const h = Math.round((w * 9) / 16);
  return `https://static-cdn.jtvnw.net/previews-ttv/live_user_${encodeURIComponent(login.toLowerCase())}-${w}x${h}.jpg?t=${Math.floor(now / 60_000)}`;
}

export interface PreviewFacts {
  live: boolean;
  rerun: boolean;
  title: string;
  /** Category, or '' */
  game: string;
  /** "12.345 watching", or '' offline. */
  viewers: string;
  /** "3h 30m", or '' offline. */
  uptime: string;
  /** The picture: the live thumbnail, the offline banner, or null (show the avatar). */
  image: string | null;
}

export function previewFacts(c: ChannelInfo, cssWidth: number, dpr: number, now = Date.now()): PreviewFacts {
  const s = c.stream;
  if (!s) {
    return { live: false, rerun: false, title: '', game: '', viewers: '', uptime: '', image: c.offlineImage ?? null };
  }
  return {
    live: true,
    rerun: s.rerun,
    title: s.title.trim(),
    game: s.game ?? '',
    viewers: `${formatNumber(s.viewers)} watching`,
    uptime: uptimeShort(s.startedAt, now),
    image: previewUrl(c.login, cssWidth, dpr, now),
  };
}
