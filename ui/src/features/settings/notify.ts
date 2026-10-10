// 2.2 Twitch's own notification switches, grouped and named the way twitch.tv's settings
// page does. Twitch sends categories by code; unknown ones land in "Other".

import type { TwitchNotifySwitch } from '../../ipc/bindings/TwitchNotifySwitch';

export const PLATFORMS = [
  { id: 'onsite', label: 'On Twitch' },
  { id: 'email', label: 'Email' },
  { id: 'push', label: 'Phone' },
] as const;

const GROUPS: [string, [string, string][]][] = [
  [
    'Channels you follow',
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
    [
      ['STREAMSTATS', 'Stream stats'],
      ['STREAMSTATSTOPCLIPS', 'Top clips of your streams'],
      ['STREAMER', 'Streamer tips'],
      ['CREATOR', 'Creator tips'],
      ['CREATORUPDATES', 'Creator updates'],
      ['CREATORSPONSORSHIPS', 'Sponsorships'],
      ['CREATORSPONSORSHIPSFROMTHIRDPARTIES', 'Sponsorships from partners'],
      ['MODERATOR', 'Moderation'],
    ],
  ],
  [
    'Developers',
    [
      ['3PDEVELOPERS', 'Extensions and apps'],
      ['DEVELOPERPRODUCTUPDATES', 'Developer news'],
    ],
  ],
];

export interface Category {
  id: string;
  label: string;
  /** Platform id → on. Missing when Twitch has no switch for that platform. */
  on: Partial<Record<string, boolean>>;
}

export interface Group {
  label: string;
  categories: Category[];
}

/** Twitch's switches as groups of categories. Master switches (`ALL`, `*_category`) are left out. */
export function group(switches: TwitchNotifySwitch[]): Group[] {
  const byCat = new Map<string, Category>();
  for (const s of switches) {
    if (s.category === 'ALL' || s.category.endsWith('_category')) continue;
    if (!PLATFORMS.some((p) => p.id === s.platform)) continue;
    const c = byCat.get(s.category) ?? { id: s.category, label: '', on: {} };
    c.on[s.platform] = s.on;
    byCat.set(s.category, c);
  }
  const out: Group[] = [];
  for (const [label, cats] of GROUPS) {
    const categories = cats.flatMap(([id, name]) => {
      const c = byCat.get(id);
      if (!c) return [];
      byCat.delete(id);
      return [{ ...c, label: name }];
    });
    if (categories.length) out.push({ label, categories });
  }
  const rest = [...byCat.values()].map((c) => ({ ...c, label: pretty(c.id) }));
  if (rest.length) out.push({ label: 'Other', categories: rest });
  return out;
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
