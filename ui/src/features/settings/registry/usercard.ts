// V111-13 Settings > Usercard: one switch per thing a usercard shows. Off also skips its lookup.

import type { UsercardShow } from '../../../ipc/bindings/UsercardShow';
import type { Entry } from './entry';

export interface UsercardItem {
  key: keyof UsercardShow;
  label: string;
  hint: string;
  keywords?: string[];
}

/** In the order the card shows them. */
export const USERCARD_ITEMS: readonly UsercardItem[] = [
  { key: 'chips', label: 'Staff, Partner and Affiliate', hint: 'The chips under the name.', keywords: ['partner', 'affiliate', 'staff'] },
  { key: 'verifiedBot', label: 'Verified bot', hint: 'A chip for bots Twitch verified. Looked up on api.ivr.fi.', keywords: ['bot'] },
  { key: 'paint', label: '7TV paint name', hint: 'The name of their 7TV paint.', keywords: ['7tv', 'paint'] },
  { key: 'badges', label: 'Badges', hint: 'Every Twitch badge they own in the channel.', keywords: ['badges'] },
  { key: 'userId', label: 'User ID', hint: 'Their Twitch ID, with a copy button.', keywords: ['id'] },
  { key: 'seventvProfile', label: '7TV profile button', hint: 'Opens their 7TV page.', keywords: ['7tv'] },
  { key: 'follow', label: 'Follow button', hint: 'Follow or unfollow as your account. Needs its Twitch sign-in.', keywords: ['follow', 'unfollow', 'heart'] },
  { key: 'created', label: 'Account created', hint: 'The day the account was made.', keywords: ['age', 'created'] },
  { key: 'followers', label: 'Followers', hint: 'How many follow them.', keywords: ['followers'] },
  { key: 'lastLive', label: 'Last live', hint: 'When they last streamed, with its title. Looked up on api.ivr.fi.', keywords: ['stream', 'live', 'title'] },
  { key: 'sub', label: 'Sub status', hint: 'Tier and months in this channel. Looked up on api.ivr.fi.', keywords: ['subscriber', 'months', 'tier'] },
  { key: 'gifter', label: 'Who gifted the sub', hint: 'Next to the sub status.', keywords: ['gift', 'gifted'] },
  { key: 'followAge', label: 'Follow age', hint: 'Since when they follow. Only where an account here moderates.', keywords: ['following since'] },
  { key: 'names', label: 'Earlier names', hint: 'From logs.cyex.app.', keywords: ['name history', 'renamed'] },
  { key: 'chatColor', label: 'Chat color', hint: 'Their color as a hex code, with a copy button.', keywords: ['colour', 'hex', 'name color'] },
  { key: 'otherModCounts', label: 'Bans elsewhere', hint: 'Bans and timeouts in other channels you moderate.', keywords: ['bans', 'timeouts', 'moderation'] },
  { key: 'roles', label: 'Roles in other channels', hint: 'Where they are mod, VIP, founder or artist. Looked up on roles.tv.', keywords: ['mod', 'vip', 'founder', 'artist', 'roles.tv'] },
  { key: 'modTabs', label: 'Mod tabs', hint: 'Timeouts, bans, warnings and comments, where you moderate.', keywords: ['mod logs', 'comments', 'warnings'] },
];

export const USERCARD: Entry[] = USERCARD_ITEMS.map((i) => ({
  id: `chat.usercard.${i.key}`,
  page: 'usercard',
  label: i.label,
  description: i.hint,
  keywords: ['usercard', 'user card', ...(i.keywords ?? [])],
}));
