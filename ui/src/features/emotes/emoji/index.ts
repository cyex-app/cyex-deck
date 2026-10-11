// E1 Emoji for the emote menu and `:name:` completion, and V102-07 their names on hover.
// The list loads on first use.

import { emojiKey } from './split.ts';

export interface Emoji {
  char: string;
  /** Short names, first one is the main one. */
  names: string[];
  /** Extra search words. */
  words: string;
}

export interface EmojiGroup {
  title: string;
  emoji: Emoji[];
}

const TITLES: Record<string, string> = {
  'Smileys & Emotion': 'Smileys',
  'People & Body': 'People',
  'Animals & Nature': 'Nature',
  'Food & Drink': 'Food',
  'Travel & Places': 'Travel',
};

let groups: Promise<EmojiGroup[]> | null = null;

export function parseGroups(raw: [string, string][]): EmojiGroup[] {
  return raw.map(([category, lines]) => ({
    title: TITLES[category] ?? category,
    emoji: lines.split('\n').map((line) => {
      const space = line.indexOf(' ');
      const [names, words = ''] = line.slice(space + 1).split('|');
      return { char: line.slice(0, space), names: names.split(' '), words };
    }),
  }));
}

export function emojiGroups(): Promise<EmojiGroup[]> {
  groups ??= import('./data').then((m) => parseGroups(m.EMOJI));
  return groups;
}

/** V102-07 What an emoji is called on hover: its short name as words, no colons ("eyes",
 * "heart eyes"). A wordy name beats `+1`; flags read "flag germany", not "de". */
export function emojiLabel(e: Emoji): string {
  const name = e.names.find((n) => /[a-z]{3}/.test(n));
  if (name) return name.replaceAll('_', ' ');
  return e.words || e.names[0];
}

/** Hover names by emoji, built from the same list. */
export function nameMap(all: EmojiGroup[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const g of all) {
    for (const e of g.emoji) {
      const key = emojiKey(e.char);
      if (!map.has(key)) map.set(key, emojiLabel(e));
    }
  }
  return map;
}

let names: Promise<Map<string, string>> | null = null;

/** The hover name for an emoji as found in chat, or null when it isn't in the list. */
export async function emojiName(char: string): Promise<string | null> {
  names ??= emojiGroups().then(nameMap);
  return (await names).get(emojiKey(char)) ?? null;
}

/** Names starting with `q` first, then ones containing it, then search words. */
export function findEmoji(all: EmojiGroup[], q: string, limit = 60): Emoji[] {
  const p = q.toLowerCase();
  if (!p) return [];
  const starts: Emoji[] = [];
  const contains: Emoji[] = [];
  const tagged: Emoji[] = [];
  for (const g of all) {
    for (const e of g.emoji) {
      if (e.names.some((n) => n.startsWith(p))) starts.push(e);
      else if (e.names.some((n) => n.includes(p))) contains.push(e);
      else if (e.words.includes(p)) tagged.push(e);
    }
  }
  return [...starts, ...contains, ...tagged].slice(0, limit);
}
