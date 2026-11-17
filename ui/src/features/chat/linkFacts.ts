// V108-C25 The words on a link's card: its kind and its facts, in the app's formats
// (dates yyyy/mm/dd, numbers with dots; 3.1, 3.13).

import type { LinkPreview } from '../../ipc/bindings/LinkPreview';
import type { PreviewFact } from '../../ipc/bindings/PreviewFact';
import { formatDate, formatLength, formatNumber } from '../../lib/format.ts';

const count = (n: number, one: string, many: string) => `${formatNumber(n)} ${n === 1 ? one : many}`;

/** One fact in words. `site` picks the site's own word: Reddit counts upvotes, X replies.
 *  V111-58 Lengths read as a clock (9:06, 1:02:03) everywhere. */
export function factText(f: PreviewFact, site = ''): string {
  switch (f.t) {
    case 'views':
      return count(f.n, 'view', 'views');
    case 'likes':
      return site === 'Reddit' ? count(f.n, 'upvote', 'upvotes') : count(f.n, 'like', 'likes');
    case 'comments':
      return site === 'X' ? count(f.n, 'reply', 'replies') : count(f.n, 'comment', 'comments');
    case 'reposts':
      return count(f.n, 'repost', 'reposts');
    case 'viewers':
      return count(f.n, 'viewer', 'viewers');
    case 'length':
      return formatLength(f.secs);
    case 'date': {
      const d = new Date(f.at);
      return Number.isNaN(d.getTime()) ? '' : formatDate(d);
    }
    case 'media':
      return [f.photos && count(f.photos, 'photo', 'photos'), f.videos && count(f.videos, 'video', 'videos')].filter(Boolean).join(', ');
    case 'spoiler':
      return 'Spoiler';
    case 'author':
    case 'community':
    case 'game':
      return f.name;
    case 'clippedBy':
      return `Clipped by ${f.name}`;
  }
}

/** One labelled line of a card's facts. `label` null: the text says it ("148.100 likes"). */
export interface FactRow {
  label: string | null;
  text: string;
  num: boolean;
}

/** V111-57 V111-58 Chatterino's order: who and where, then length, date, counts, flags. */
const ROW_ORDER: PreviewFact['t'][] = ['author', 'community', 'length', 'date', 'views', 'likes', 'reposts', 'comments', 'media', 'spoiler'];

/** V111-57 V111-58 A card from our resolver, drawn as labelled lines like Chatterino's tip:
 *  Channel (YouTube) or Posted by and Subreddit, Duration (a clock), Published, Views, then
 *  "148.100 likes", reposts, comments, Media and Spoiler. `sub` is the subtitle still worth showing
 *  under the title (null when a row already says it). Null for Twitch cards and cards without
 *  facts (Chatterino's), which keep their own look. */
export function factRows(p: LinkPreview): { rows: FactRow[]; sub: string | null } | null {
  if (p.kind !== 'web') return null;
  const channel = p.site === 'YouTube' && p.subtitle ? p.subtitle : null;
  if (!p.facts.length && !channel) return null;
  const rows: FactRow[] = channel ? [{ label: 'Channel', text: channel, num: false }] : [];
  const facts = [...p.facts].sort((a, b) => rank(a) - rank(b));
  for (const f of facts) {
    // "Views: 6.234.567": the label says what the number counts.
    const text = f.t === 'views' ? formatNumber(f.n) : factText(f, p.site);
    if (text) rows.push({ label: rowLabel(f, p.site), text, num: numeric(f) });
  }
  const said = channel || facts.some((f) => f.t === 'author' || f.t === 'community');
  return { rows, sub: said ? null : p.subtitle };
}

const rank = (f: PreviewFact) => {
  const i = ROW_ORDER.indexOf(f.t);
  return i < 0 ? ROW_ORDER.length : i;
};

function rowLabel(f: PreviewFact, site: string): string | null {
  switch (f.t) {
    case 'author':
      return 'Posted by';
    case 'community':
      return site === 'Reddit' ? 'Subreddit' : 'Community';
    case 'length':
      return 'Duration';
    case 'date':
      return 'Published';
    case 'views':
      return 'Views';
    case 'media':
      return 'Media';
    default:
      return null;
  }
}

/** Counts, lengths and dates are data: drawn in mono. Names, media and flags are words. */
export const numeric = (f: PreviewFact) => !['game', 'clippedBy', 'author', 'community', 'media', 'spoiler'].includes(f.t);

/** V111-58 Facts the inline card's site line leaves out: the subtitle line already names who and where. */
export const inlineFact = (f: PreviewFact) => f.t !== 'author' && f.t !== 'community';

/** The small line over the title, e.g. "Twitch clip". */
export function kindText(p: LinkPreview): string {
  switch (p.kind) {
    case 'clip':
      return 'Twitch clip';
    case 'video':
      return 'Twitch video';
    case 'channel':
      return 'Twitch channel';
    case 'web':
      return p.site;
  }
}
