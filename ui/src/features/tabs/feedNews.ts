// V107-H15 News in feed tabs (the Mentions tab) nobody is looking at: lines counted per feed
// until its tab is shown.

import type { Tab } from '../../ipc/bindings/Tab';
import { isMentionsFeed } from '../chat/mentionsTab.ts';

export type FeedCounts = Record<string, number>;

/** The key Rust files a feed under. */
export const feedKey = (name: string) => name.trim().toLowerCase();

/** The feed names a tab shows. */
export function feedsOf(tab: Tab): string[] {
  return tab.columns.flatMap((c) => c.tiles.flatMap((t) => (t.feed ? [feedKey(t.feed)] : [])));
}

/** `counts` with one more line in `feed`, unless a tab shows it (then they're unchanged). */
export function bumped(counts: FeedCounts, feed: string, shown: Iterable<Tab>): FeedCounts {
  for (const tab of shown) if (feedsOf(tab).includes(feed)) return counts;
  return { ...counts, [feed]: (counts[feed] ?? 0) + 1 };
}

/** `counts` without the feeds this tab shows. */
export function cleared(counts: FeedCounts, tab: Tab): FeedCounts {
  const feeds = feedsOf(tab).filter((f) => counts[f]);
  if (!feeds.length) return counts;
  const next = { ...counts };
  for (const f of feeds) delete next[f];
  return next;
}

/** Unseen lines in the Mentions feed shown by `tab`. Other feeds stay quiet. */
export function mentionNews(counts: FeedCounts, tab: Tab): number {
  let n = 0;
  for (const f of new Set(feedsOf(tab))) if (isMentionsFeed(f)) n += counts[f] ?? 0;
  return n;
}
