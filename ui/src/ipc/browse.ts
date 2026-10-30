import { invoke } from '@tauri-apps/api/core';

import type { Category } from './bindings/Category';
import type { CategoryStreams } from './bindings/CategoryStreams';
import type { ChannelCard } from './bindings/ChannelCard';
import type { ChannelSchedule } from './bindings/ChannelSchedule';
import type { NextStream } from './bindings/NextStream';
import type { Page } from './bindings/Page';
import type { SearchResults } from './bindings/SearchResults';
import type { StreamFilter } from './bindings/StreamFilter';

export const homeIpc = {
  /** 1.6 The account's follows. `force` fetches the whole list again. */
  follows: (account: string, force = false) => invoke<ChannelCard[]>('home_follows', { account, force }),
  /** H2 Channels by user id. */
  channels: (ids: string[]) => invoke<ChannelCard[]>('home_channels', { ids }),
};

export const discoverIpc = {
  streams: (filter: StreamFilter, after: string | null = null) =>
    invoke<Page<ChannelCard>>('discover_streams', { filter, after }),
  categories: (after: string | null = null) => invoke<Page<Category>>('discover_categories', { after }),
  /** `null` if there's no such category. */
  category: (slug: string, filter: StreamFilter, after: string | null = null) =>
    invoke<CategoryStreams | null>('discover_category', { slug, filter, after }),
  search: (text: string) => invoke<SearchResults>('discover_search', { text }),
  searchMore: (text: string, after: string) => invoke<Page<ChannelCard>>('discover_search_more', { text, after }),
  /** H4 Empty without a GQL sign-in. */
  followedCategories: (account: string) => invoke<Category[]>('discover_followed_categories', { account }),
};

export const scheduleIpc = {
  /** V108-N03 A channel's upcoming streams and vacation. */
  channel: (login: string) => invoke<ChannelSchedule>('schedule_channel', { login }),
  /** V108-N03 The next planned stream of each channel (by user id) that has one. */
  next: (ids: string[]) => invoke<NextStream[]>('schedule_next', { ids }),
  /** V108-N03 Saves the schedule as an .ics file; the path, or null when cancelled. */
  saveIcs: (login: string, name: string) => invoke<string | null>('schedule_save_ics', { login, name }),
};
