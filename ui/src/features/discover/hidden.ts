// V108-N05 Categories and tags left out of Home and Discover. Pure, so it's tested without
// Svelte. Categories are matched by Twitch's id (a live stream carries it), else by name;
// tags in any case.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import type { HiddenCategory } from '../../ipc/bindings/HiddenCategory';

export interface Hidden {
  categories: readonly HiddenCategory[];
  tags: readonly string[];
}

/** The most the settings keep (the Rust side trims to the same). */
export const MAX_HIDDEN = 300;

const key = (s: string) => s.trim().toLowerCase();

export function categoryHidden(h: Hidden, c: { id?: string | null; name?: string | null }): boolean {
  if (!h.categories.length) return false;
  const name = c.name ? key(c.name) : '';
  return h.categories.some((x) => (!!c.id && x.id === c.id) || (!!name && key(x.name) === name));
}

export function tagHidden(h: Hidden, tags: readonly string[]): boolean {
  if (!h.tags.length || !tags.length) return false;
  const set = new Set(h.tags.map(key));
  return tags.some((t) => set.has(key(t)));
}

/** A live channel to leave out: its category or one of its tags is hidden. `game: false`
 *  skips the category, e.g. inside the category page itself. Offline channels stay. */
export function cardHidden(h: Hidden, card: ChannelCard, { game = true } = {}): boolean {
  const s = card.stream;
  if (!s) return false;
  return (game && !!s.game && categoryHidden(h, s.game)) || tagHidden(h, s.tags);
}

export const visibleCards = (h: Hidden, cards: ChannelCard[], opts?: { game?: boolean }): ChannelCard[] =>
  !h.categories.length && !h.tags.length ? cards : cards.filter((c) => !cardHidden(h, c, opts));

export const visibleCategories = <T extends { id: string; name: string }>(h: Hidden, cats: T[]): T[] =>
  h.categories.length ? cats.filter((c) => !categoryHidden(h, c)) : cats;

/** The list with `cat` added (no duplicates, newest last, capped). */
export function withHidden(list: readonly HiddenCategory[], cat: HiddenCategory): HiddenCategory[] {
  if (!cat.id || list.some((x) => x.id === cat.id)) return [...list];
  return [...list, { id: cat.id, name: cat.name }].slice(-MAX_HIDDEN);
}

export const withoutHidden = (list: readonly HiddenCategory[], id: string): HiddenCategory[] =>
  list.filter((x) => x.id !== id);

/** The tags with `text` added, unless it is empty or there already (any case). */
export function withTag(tags: readonly string[], text: string): string[] {
  const t = text.trim().replace(/^#/, '').trim();
  if (!t || tags.some((x) => key(x) === key(t))) return [...tags];
  return [...tags, t];
}
