// 2.2 Twitch's own notification switches, grouped and named the way twitch.tv's settings
// page does. Each group has a master switch per platform, and `ALL` turns a whole platform
// off (V101-06). Twitch sends categories by code; unknown ones land in "Other".

import type { TwitchNotifySwitch } from '../../ipc/bindings/TwitchNotifySwitch';

export const PLATFORMS = [
  { id: 'onsite', label: 'On Twitch' },
  { id: 'email', label: 'Email' },
  { id: 'push', label: 'Phone' },
] as const;

const GROUPS: [string, string, [string, string][]][] = [
  [
    'Channels you follow',
    'following_category',
    [
      ['LIVE', 'Going live'],
      ['CATCHUP', 'Streams you missed'],
      ['RECOMMENDEDLIVE', 'Live streams you might like'],
      ['VIDEOS', 'New videos'],
      ['NEWSTORIES', 'New stories'],
      ['SUBSCRIPTIONS', 'Subscriptions'],
    ],
  ],
  [
    'People',
    'social_category',
    [
      ['CHAT', 'Chat mentions'],
      ['FOLLOWERADD', 'New followers'],
      ['FRIENDS', 'Friends'],
      ['GIFTSUBSCRIPTIONS', 'Gifted subs'],
      ['SUB_MESSAGES', 'Sub messages'],
      ['CLIPS', 'Clips'],
      ['CLIPCOMMENTS', 'Clip comments'],
      ['CLIPSHAREDINSTORY', 'Your clips in stories'],
      ['MENTIONEDINSTORY', 'Mentions in stories'],
      ['STORIESREACTION', 'Story reactions'],
      ['GUESTSTAR', 'Stream Together'],
      ['STREAMERSHOUTS', 'Shoutouts'],
    ],
  ],
  [
    'From Twitch',
    'marketing_category',
    [
      ['WATCHSTREAKS', 'Watch streaks'],
      ['DROPSANDQUESTS', 'Drops and quests'],
      ['BADGESANDEMOTES', 'Badges and emotes'],
      ['EVENTS', 'Events'],
      ['COMMERCE', 'Purchases'],
      ['RECOMMENDEDCONTENT', 'Recommendations'],
      ['PRODUCTUPDATES', 'Twitch news'],
      ['USERENDOFYEAR', 'Your year on Twitch'],
      ['PROMOTIONS', 'Offers'],
      ['PROMOTIONSFROMTHIRDPARTIES', 'Offers from partners'],
      ['MARKETING', 'Marketing'],
      ['RESEARCHSTUDIES', 'Research studies'],
    ],
  ],
  [
    'Your channel',
    'creator_category',
    [
      ['STREAMSTATS', 'Stream stats'],
      ['STREAMSTATSTOPCLIPS', 'Top clips of your streams'],
      ['STREAMER', 'Streamer tips'],
      ['CREATOR', 'Creator tips'],
      ['CREATORUPDATES', 'Creator updates'],
      ['CREATORSPONSORSHIPS', 'Sponsorships'],
      ['CREATORSPONSORSHIPSFROMTHIRDPARTIES', 'Sponsorships from partners'],
    ],
  ],
  ['Moderation', 'moderation_category', [['MODERATOR', 'Moderation']]],
  [
    'Developers',
    'developer_category',
    [
      ['3PDEVELOPERS', 'Extensions and apps'],
      ['DEVELOPERPRODUCTUPDATES', 'Developer news'],
    ],
  ],
];

const EVERYTHING = 'ALL';

export interface Category {
  id: string;
  label: string;
  /** Platform id → on. Missing when Twitch has no switch for that platform. */
  on: Partial<Record<string, boolean>>;
}

export interface Group {
  label: string;
  /** Twitch's switch for the whole group, if it sent one. */
  master: Category | null;
  categories: Category[];
}

export interface Switches {
  /** `ALL`: a whole platform on or off. */
  all: Category | null;
  groups: Group[];
}

/** Twitch's switches as groups of categories under their master switches. */
export function group(switches: TwitchNotifySwitch[]): Switches {
  const byCat = new Map<string, Category>();
  for (const s of switches) {
    if (!PLATFORMS.some((p) => p.id === s.platform)) continue;
    const c = byCat.get(s.category) ?? { id: s.category, label: '', on: {} };
    c.on[s.platform] = s.on;
    byCat.set(s.category, c);
  }
  const take = (id: string, label: string): Category | null => {
    const c = byCat.get(id);
    byCat.delete(id);
    return c ? { ...c, label } : null;
  };
  const all = take(EVERYTHING, 'Everything');
  const groups: Group[] = [];
  for (const [label, masterId, cats] of GROUPS) {
    const master = take(masterId, label);
    const categories = cats.flatMap(([id, name]) => take(id, name) ?? []);
    if (categories.length) groups.push({ label, master, categories });
  }
  // Masters Twitch adds later have nothing to switch here.
  const rest = [...byCat.values()].filter((c) => !c.id.endsWith('_category')).map((c) => ({ ...c, label: pretty(c.id) }));
  if (rest.length) groups.push({ label: 'Other', master: null, categories: rest });
  return { all, groups };
}

/** Whether Twitch sends a group's notifications on a platform: its master and `ALL` both on. */
export function sends(s: Switches, g: Group | null, platform: string): boolean {
  return s.all?.on[platform] !== false && g?.master?.on[platform] !== false;
}

/** How many of a group's switches are on for a platform, for the closed row. */
export function countOn(g: Group, platform: string): [number, number] {
  const has = g.categories.filter((c) => c.on[platform] !== undefined);
  return [has.filter((c) => c.on[platform]).length, has.length];
}

function pretty(code: string): string {
  const s = code.replace(/_/g, ' ').toLowerCase();
  return s.charAt(0).toUpperCase() + s.slice(1);
}

/** Twitch's per-channel choice, in its own words. */
export const CHANNEL_STATES = [
  { value: 'ALWAYS', label: 'All' },
  { value: 'LIVE_UP_ONLY', label: 'Going live' },
  { value: 'PERSONALIZED', label: 'Twitch picks' },
  { value: 'NEVER', label: 'Off' },
] as const;
