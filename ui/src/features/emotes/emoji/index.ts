// E1 Emoji for the emote menu and `:name:` completion. The list loads on first use.

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
