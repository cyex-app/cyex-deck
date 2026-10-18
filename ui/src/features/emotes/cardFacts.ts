// 1.41 What the emote card lists, in reading order: where it's from, who made it, who
// added it, when, which set. Links only where the card can follow them (in-app channel,
// the providers' own sites).
import type { CardFlag } from '../../ipc/bindings/CardFlag';
import type { EmoteCard } from '../../ipc/bindings/EmoteCard';
import type { Provider } from '../../ipc/bindings/Provider';
import { formatDate, formatNumber, formatTime } from '../../lib/format.ts';

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
  /** kb-3 Never folds into "+N" (warnings, private). */
  keep?: boolean;
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

/** kb-2 The card's short date and its tip, both local: `2026/10/18`, `2026/10/18 14:05`. */
export function addedDate(iso: string | null): { value: string; tip: string } | null {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return { value: formatDate(d), tip: `${formatDate(d)} ${formatTime(d)}` };
}

/** ka-6 The emote's page when the card's details didn't load, as the app builds it. */
export function emotePage(provider: Provider, id: string, name: string): string {
  switch (provider) {
    case 'seventv':
      return `https://7tv.app/emotes/${encodeURIComponent(id)}`;
    case 'bttv':
      return `https://betterttv.com/emotes/${encodeURIComponent(id)}`;
    case 'ffz':
      return `https://www.frankerfacez.com/emoticon/${encodeURIComponent(`${id}-${name}`)}`;
    case 'twitch':
      return `https://chatvau.lt/emote/twitch/${encodeURIComponent(id)}`;
  }
}

const PROFILE: Partial<Record<Provider, string>> = { seventv: '7TV profile', bttv: 'BTTV profile' };

export function cardFacts(info: EmoteCard, provider: Provider): Fact[] {
  const added = addedDate(info.addedAt);
  const out: Fact[] = [];
  // A Twitch global belongs to no channel worth showing; sub, follower and bits emotes
  // open theirs in the app.
  const global = provider === 'twitch' && info.flags.includes('global');
  if (info.channel && !global) out.push(channel('Channel', info.channel, info.channelLogin, info.channelAvatar));
  if (info.owner) {
    out.push(
      provider === 'twitch' ? channel('Artist', info.owner, info.artistLogin) : web('By', info.owner, info.ownerUrl),
    );
  } else if (PROFILE[provider] && info.ownerUrl) {
    // ct-11, ka-1 The owner’s name didn’t come with the emote (BTTV never sends it now);
    // the profile still opens.
    out.push(web('By', PROFILE[provider], info.ownerUrl));
  }
  if (info.addedBy) out.push({ label: 'Added by', value: info.addedBy });
  if (added) out.push({ label: provider === 'ffz' ? 'Uploaded' : 'Added', ...added });
  if (info.setUrl) out.push(web('Set', setName(info.flags), info.setUrl));
  return out;
}

/** fb-4, fa-2 The facts a card usually gets, for the lines held while it loads: 7TV by,
 *  added by, added, set; BTTV by; FFZ by, uploaded; a Twitch channel emote (a lowercase
 *  prefix, `lirikH`) its channel and artist, a global (`LUL`) none. */
export function expectedFacts(provider: Provider, name: string): number {
  switch (provider) {
    case 'seventv':
      return 4;
    case 'bttv':
      return 1;
    case 'ffz':
      return 2;
    case 'twitch':
      return /^[a-z][a-z0-9]{2,}[A-Z0-9_]/.test(name) ? 2 : 0;
  }
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
/** kb-3, fa-3 Where it works (Shared too), first after the provider. */
const SCOPE: CardFlag[] = ['global', 'channel', 'personal', 'shared'];
/** kb-3 Shown with the warnings and never folded, but not in yellow. */
const KEEP: CardFlag[] = ['private'];

function flagChip(f: CardFlag, info: EmoteCard): Chip {
  let [text, tip] = FLAGS[f];
  if (f === 'channel' && info.provider === 'twitch') tip = 'Belongs to a channel';
  if (f === 'bits' && info.bits) {
    const n = formatNumber(info.bits);
    text = `${n} bits`;
    tip = `Unlocked by cheering ${n} bits in the channel`;
  }
  if (f === 'aliased' && info.original) tip = `Renamed in this channel. Its own name is ${info.original}`;
  if (WARN.includes(f)) return { text, tip, warn: true, keep: true };
  return KEEP.includes(f) ? { text, tip, keep: true } : { text, tip };
}

/** kb-3 The chips after the provider's: scope, then the warnings (the state among them),
 *  then the rest in the card's order and gigantified. */
export function cardChips(info: EmoteCard | null, giant: boolean): Chip[] {
  const flags = info?.flags ?? [];
  const scope = flags.filter((f) => SCOPE.includes(f));
  const rest = flags.filter((f) => !SCOPE.includes(f));
  const chips = info ? [...scope, ...rest].map((f) => flagChip(f, info)) : [];
  if (info?.state) chips.push({ text: info.state, tip: `Twitch lists it as ${info.state.toLowerCase()}`, warn: true, keep: true });
  if (giant) chips.push({ text: 'Gigantified', tip: 'Made bigger in this message with channel points' });
  const lead = chips.slice(0, scope.length);
  const later = chips.slice(scope.length);
  return [...lead, ...later.filter((c) => c.keep), ...later.filter((c) => !c.keep)];
}

/** R7 The chip row’s room: the card’s 300 px less its padding. */
const ROW = 276;
const GAP = 4;
/** A chip’s width at 0.75rem, padding and border included; a little generous. The card
 *  passes measured text widths instead. */
export const chipWidth = (text: string) => Math.ceil(text.length * 6.6) + 16;

/** Lines the chips take, packed like the flex row does. */
function lines(texts: string[], width: number, measure: (text: string) => number): number {
  let n = 1;
  let x = 0;
  for (const t of texts) {
    const w = measure(t);
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

/** R7, kb-3 The chips that fit in two lines of `width` after the provider’s (`lead`); the
 *  rest fold into "+N", from the end, which only appears when it hides two or more. Kept
 *  chips (warnings) never fold. */
export function foldChips(chips: Chip[], lead: string, width = ROW, measure = chipWidth): Folded {
  const fits = (shown: Chip[], plus?: string) =>
    lines([lead, ...shown.map((c) => c.text), ...(plus ? [plus] : [])], width, measure) <= 2;
  if (fits(chips)) return { shown: chips, rest: [] };
  const foldable = chips.filter((c) => !c.keep);
  if (foldable.length < 2) return { shown: chips, rest: [] };
  for (let k = foldable.length - 2; k >= 0; k--) {
    const rest = foldable.slice(k);
    const shown = chips.filter((c) => !rest.includes(c));
    if (fits(shown, `+${rest.length}`) || k === 0) return { shown, rest };
  }
  return { shown: chips, rest: [] };
}
