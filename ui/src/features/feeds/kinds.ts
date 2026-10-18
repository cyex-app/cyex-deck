// U8 Everything that describes a feed kind, in one place: the add-tab choices and the feed tab's
// own cap, icon and empty text. A new kind fails to compile until it is described here.

import type { IconName } from '../../components/icons';
import type { FeedKind } from '../../ipc/bindings/FeedKind';
import { isMentionsFeed, MENTIONS_FEED } from '../chat/mentionsTab.ts';

export type FeedChoice = 'mentions' | 'live' | 'automod' | 'whispers' | 'custom';

export const FEED_CHOICES: { value: FeedChoice; label: string; note: string }[] = [
  { value: 'mentions', label: 'Mentions', note: 'Your mentions and replies, from every open chat.' },
  {
    value: 'live',
    label: 'Live notifications',
    note: 'Who goes live or offline: your tabs, and the channels you follow while live alerts are on.',
  },
  { value: 'automod', label: 'AutoMod', note: 'Held messages from the chats you moderate. Allow or deny them here.' },
  { value: 'whispers', label: 'Whispers', note: 'Whispers you get while cyex deck runs.' },
  { value: 'custom', label: 'Custom feed', note: 'Messages from highlight rules set to this feed.' },
];

/** What a feed tab of each own kind (not the rules feeds) shows. */
export const KIND: Record<Exclude<FeedKind, 'rules'>, { cap: number; icon: IconName; empty: string }> = {
  live: { cap: 300, icon: 'radio', empty: 'Go-live and offline alerts show up here.' },
  automod: { cap: 200, icon: 'shield', empty: 'Nothing held right now. Held messages from chats you moderate show up here.' },
  whispers: { cap: 200, icon: 'whisper', empty: 'No whispers yet this session.' },
};

export interface FeedSpec {
  kind: FeedKind;
  name: string;
}

/** The tab a choice makes. Mentions and custom feeds are rules feeds named by their feed. */
export function feedSpec(choice: FeedChoice, name: string): FeedSpec {
  if (choice === 'mentions') return { kind: 'rules', name: MENTIONS_FEED };
  if (choice === 'custom') return { kind: 'rules', name: name.trim() };
  return { kind: choice, name: '' };
}

/** Whether a tile already is this feed. A custom feed with no name is none. */
export function sameFeed(tile: { feed: string | null; feedKind: FeedKind }, spec: FeedSpec): boolean {
  if (spec.kind !== 'rules') return !!tile.feed && tile.feedKind === spec.kind;
  if (tile.feedKind !== 'rules') return false;
  if (isMentionsFeed(spec.name)) return isMentionsFeed(tile.feed);
  return !!spec.name && tile.feed?.trim().toLowerCase() === spec.name.toLowerCase();
}
