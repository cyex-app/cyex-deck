// 1.41 What the emote card lists, in reading order: where it's from, who made it, who
// added it, when, which set. Links only where the card can follow them (in-app channel,
// the providers' own sites).
import type { CardFlag } from '../../ipc/bindings/CardFlag';
import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import type { Provider } from '../../ipc/bindings/Provider';
import { formatNumber } from '../../lib/format.ts';

export type Fact =
  /** `tip`: the full date behind a short one. */
  | { label: string; value: string; link?: undefined; tip?: string }
  /** Opens the channel's tab in the app. `avatar`: the channel's picture. */
  | { label: string; value: string; link: 'channel'; login: string; avatar?: string }
  /** Opens a page in the browser. */
  | { label: string; value: string; link: 'web'; url: string; tip: string };

export interface Chip {
  text: string;
  tip: string;
  warn?: boolean;
}

/** The hostname without `www.`, for "Opens 7tv.app". */
function host(url: string): string {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return '';
  }
}

/** "Opens 7tv.app", or `null` for a url that isn't one. */
export function opensTip(url: string | null): string | null {
  const at = url ? host(url) : '';
  return at ? `Opens ${at}` : null;
}

function web(label: string, value: string, url: string | null): Fact {
  const tip = opensTip(url);
  return url && tip ? { label, value, link: 'web', url, tip } : { label, value };
}

function channel(label: string, value: string, login: string | null, avatar?: string | null): Fact {
  if (!login) return { label, value };
  return avatar ? { label, value, link: 'channel', login, avatar } : { label, value, link: 'channel', login };
}

export function cardFacts(info: EmoteCard, provider: Provider, added: string | null): Fact[] {
  const out: Fact[] = [];
  if (info.channel) out.push(channel('Channel', info.channel, info.channelLogin, info.channelAvatar));
  if (info.owner) {
    out.push(
      provider === 'twitch' ? channel('Artist', info.owner, info.artistLogin) : web('By', info.owner, info.ownerUrl),
    );
  } else if (provider === 'seventv' && info.ownerUrl) {
    // ct-11 The owner’s name didn’t come with the emote; the profile still opens.
    out.push(web('By', '7TV profile', info.ownerUrl));
  }
  if (info.addedBy) out.push({ label: 'Added by', value: info.addedBy });
  if (added) out.push({ label: provider === 'ffz' ? 'Uploaded' : 'Added', value: added, tip: info.addedAt ?? undefined });
  if (info.setUrl) out.push(web('Set', setName(info.flags), info.setUrl));
  return out;
}

/** 7TV's set, named by what it's for. */
function setName(flags: CardFlag[]): string {
  if (flags.includes('personal')) return 'Personal set';
  if (flags.includes('global')) return 'Global set';
  return 'Channel set';
}

/** Words and tips for each flag. Scope tips, bits and the alias read the card for more. */
const FLAGS: Record<CardFlag, [text: string, tip: string]> = {
  global: ['Global', 'Works in every chat'],
  channel: ['Channel', 'In this channel’s set'],
  personal: ['Personal', 'From a personal set. Only its owner’s messages show it'],
  shared: ['Shared', 'Another channel’s emote, added here'],
  tier1: ['Tier 1', 'For subscribers at tier 1 and up'],
  tier2: ['Tier 2', 'For subscribers at tier 2 and up'],
  tier3: ['Tier 3', 'For tier 3 subscribers'],
  follower: ['Follower', 'For followers of the channel'],
  bits: ['Bits', 'Unlocked by cheering in the channel'],
  prime: ['Prime', 'For Prime Gaming members'],
  turbo: ['Turbo', 'For Turbo members'],
  hypeTrain: ['Hype train', 'Unlocked in a hype train'],
  twoFactor: ['Two-factor', 'For accounts with two-factor sign-in on'],
  limitedTime: ['Limited time', 'Only around for a while'],
  points: ['Points', 'Unlocked with channel points'],
  smiley: ['Smiley', 'One of Twitch’s classic smileys'],
  archivedSet: ['Archived set', 'From a set Twitch archived'],
  event: ['Event', 'From a Twitch event'],
  animated: ['Animated', 'Plays as an animation'],
  overlaying: ['Overlaying', 'Drawn over the emote before it (zero-width)'],
  private: ['Private', 'Only its owner can add it to a set'],
  unlisted: ['Unlisted', 'Hidden from 7TV’s public list'],
  personalApproved: ['Personal approved', '7TV allows it in personal sets'],
  personalDenied: ['Personal denied', '7TV keeps it out of personal sets'],
  nsfw: ['NSFW', 'Marked not safe for work on 7TV'],
  aliased: ['Renamed', 'Renamed in this channel'],
  modifier: ['Modifier', 'Changes the emote before it'],
};

const WARN: CardFlag[] = ['unlisted', 'nsfw'];

function flagChip(f: CardFlag, info: EmoteCard): Chip {
  let [text, tip] = FLAGS[f];
  if (f === 'channel' && info.provider === 'twitch') tip = 'Belongs to a channel';
  if (f === 'bits' && info.bits) {
    const n = formatNumber(info.bits);
    text = `${n} bits`;
    tip = `Unlocked by cheering ${n} bits in the channel`;
  }
  if (f === 'aliased' && info.original) tip = `Renamed in this channel. Its own name is ${info.original}`;
  return WARN.includes(f) ? { text, tip, warn: true } : { text, tip };
}

/** The chips after the provider's, in the card's order, then gigantified and the state. */
export function cardChips(info: EmoteCard | null, giant: boolean): Chip[] {
  const out: Chip[] = info ? info.flags.map((f) => flagChip(f, info)) : [];
  if (giant) out.push({ text: 'Gigantified', tip: 'Made bigger in this message with channel points' });
  if (info?.state) out.push({ text: info.state, tip: `Twitch lists it as ${info.state.toLowerCase()}`, warn: true });
  return out;
}

/** R7 At most this many chips, the provider’s included. */
export const MAX_CHIPS = 6;

/** R7 The chip row’s room: the card’s 300 px less its padding. */
const ROW = 276;
const GAP = 4;
/** A chip’s width at 0.75rem, padding and border included; a little generous. */
const chipWidth = (text: string) => Math.ceil(text.length * 6.6) + 16;

/** Lines the chips take, packed like the flex row does. */
function lines(texts: string[], width: number): number {
  let n = 1;
  let x = 0;
  for (const t of texts) {
    const w = chipWidth(t);
    if (x > 0 && x + GAP + w > width) {
      n++;
      x = w;
    } else x += (x > 0 ? GAP : 0) + w;
  }
  return n;
}

export interface Folded {
  shown: Chip[];
  /** Behind the "+N" button; never just one. */
  rest: Chip[];
}

/** R7 The chips that fit in two lines after the provider’s (`lead`); the rest fold into
 *  "+N", which only appears when it hides two or more. */
export function foldChips(chips: Chip[], lead: string, width = ROW, max = MAX_CHIPS): Folded {
  const fits = (texts: string[]) => texts.length <= max && lines(texts, width) <= 2;
  const texts = chips.map((c) => c.text);
  if (fits([lead, ...texts])) return { shown: chips, rest: [] };
  for (let k = chips.length - 2; k > 0; k--) {
    const plus = `+${chips.length - k}`;
    if (fits([lead, ...texts.slice(0, k), plus])) return { shown: chips.slice(0, k), rest: chips.slice(k) };
  }
  return { shown: [], rest: chips };
}
