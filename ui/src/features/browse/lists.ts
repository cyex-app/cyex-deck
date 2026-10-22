// Home's lists (1.6, H1, H2): favorites, then live, then offline, each sorted and filtered.
// Pure, so it's tested without Svelte.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import type { LiveSort } from '../../ipc/bindings/LiveSort';
import type { OfflineSort } from '../../ipc/bindings/OfflineSort';
import { formatDuration } from '../../lib/format.ts';

export interface Sections {
  favorites: ChannelCard[];
  live: ChannelCard[];
  offline: ChannelCard[];
}

export interface Sorting {
  live: LiveSort;
  offline: OfflineSort;
}

const byName = (a: ChannelCard, b: ChannelCard) => a.login.localeCompare(b.login);
const time = (iso: string | null | undefined) => (iso ? Date.parse(iso) || 0 : 0);

function sortLive(list: ChannelCard[], sort: LiveSort): ChannelCard[] {
  const viewers = (c: ChannelCard) => c.stream?.viewers ?? 0;
  const started = (c: ChannelCard) => time(c.stream?.startedAt);
  if (sort === 'name') return list.sort(byName);
  if (sort === 'uptime') return list.sort((a, b) => started(a) - started(b) || byName(a, b));
  return list.sort((a, b) => viewers(b) - viewers(a) || byName(a, b));
}

function sortOffline(list: ChannelCard[], sort: OfflineSort): ChannelCard[] {
  if (sort === 'name') return list.sort(byName);
  return list.sort((a, b) => time(b.lastLive) - time(a.lastLive) || byName(a, b));
}

/** Whether a channel matches what's typed in the filter box (H1). */
export function matches(c: ChannelCard, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  return (
    c.login.includes(q) ||
    c.name.toLowerCase().includes(q) ||
    !!c.stream?.game?.name.toLowerCase().includes(q)
  );
}

/** Favorites first (live ones on top), then the rest of the follows, live and offline. */
export function sections(
  follows: ChannelCard[],
  favorites: ChannelCard[],
  sorting: Sorting,
  query = '',
): Sections {
  const starred = new Set(favorites.map((c) => c.id));
  const favLive = sortLive(favorites.filter((c) => c.stream && matches(c, query)), sorting.live);
  const favOff = sortOffline(favorites.filter((c) => !c.stream && matches(c, query)), sorting.offline);
  const rest = follows.filter((c) => !starred.has(c.id) && matches(c, query));
  return {
    favorites: [...favLive, ...favOff],
    live: sortLive(rest.filter((c) => c.stream), sorting.live),
    offline: sortOffline(rest.filter((c) => !c.stream), sorting.offline),
  };
}

const DAY = 86_400_000;

/** H1 "today", "yesterday", "3 days ago", "2 weeks ago", "5 months ago", "2 years ago". */
export function since(iso: string | null | undefined, now = Date.now()): string {
  const t = time(iso);
  if (!t) return '';
  const today = new Date(now).setHours(0, 0, 0, 0);
  if (t >= today) return 'today';
  const days = Math.ceil((today - t) / DAY);
  if (days === 1) return 'yesterday';
  if (days < 14) return `${days} days ago`;
  // v1.07 Under 61 days the months read 1 ("1 months ago"): weeks until it's 2 months.
  if (days < 61) return `${Math.floor(days / 7)} weeks ago`;
  if (days < 730) return `${Math.floor(days / 30.4)} months ago`;
  return `${Math.floor(days / 365)} years ago`;
}

/** How long a stream has run, to the minute: "51m", "3h 30m", "1d 7h". */
export function uptimeShort(startedAt: string, now = Date.now()): string {
  const t = Date.parse(startedAt);
  if (Number.isNaN(t)) return '';
  // v1.07 Under a minute reads "1m", like the viewer chart, not "0s" next to the minute-only others.
  return formatDuration(Math.max(60, Math.floor(Math.max(0, now - t) / 60_000) * 60));
}

/** Twitch's preview picture. The URL is stable, so a 5-minute bucket fetches a fresh one. */
export function preview(login: string, width = 440, height = 248, now = Date.now()): string {
  return `https://static-cdn.jtvnw.net/previews-ttv/live_user_${login}-${width}x${height}.jpg?t=${Math.floor(now / 300_000)}`;
}
