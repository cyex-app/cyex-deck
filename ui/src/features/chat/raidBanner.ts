// V110-U43 The raid banner's rules: when the Join tick shows, the countdown, and what the pinned
// message does while a raid shows (folded to one line, then back the way it was).

/** The "Join" tick shows while a raid counts down, in a signed-in chat of another channel.
 * It is always there (no setting); nothing joins until it is ticked. */
export function tickShows(raidId: string | null, account: string | null, own = false): boolean {
  return !!raidId && !!account && !own;
}

/** The chat's account is the channel's broadcaster: the raid is theirs, they are the one raiding. */
export function ownChannel(account: string | null, channelId: string | null): boolean {
  return !!account && !!channelId && account === channelId;
}

/** "Cancel raid" shows in your own channel for the raid the stream manager's account runs
 * (`managerId`: the account that runs it); otherwise the call would hit another channel. */
export function cancelShows(raidId: string | null, own: boolean, managerId: string | null, channelId: string | null): boolean {
  return !!raidId && own && !!managerId && managerId === channelId;
}

/** Twitch counts down this long when a raid has no known start. */
export const RAID_SPAN_S = 90;

/** The bar: where it starts (0..1 of full), how long it runs, and the wait before it starts.
 * Unix seconds in, milliseconds out. */
export interface RaidBar {
  from: number;
  ms: number;
  delay: number;
}

export function raidBar(createdAt: number, goesAt: number, nowMs: number): RaidBar {
  const span = goesAt > createdAt ? (goesAt - createdAt) * 1000 : RAID_SPAN_S * 1000;
  const ms = Math.max(0, goesAt * 1000 - nowMs);
  const delay = Math.max(0, createdAt * 1000 - nowMs);
  const from = Math.min(1, ms / span);
  return { from: delay > 0 ? 1 : from, ms: Math.round(ms), delay: Math.round(delay) };
}

/** What the bar shows right now (reduced motion: updated each second instead of animated). */
export function raidFrac(createdAt: number, goesAt: number, nowMs: number): number {
  const b = raidBar(createdAt, goesAt, nowMs);
  return b.delay > 0 ? 1 : b.from;
}

/** The second line's left part: "Title · Category", the category alone, the title alone, or "". */
export function raidAbout(title: string | null | undefined, category: string | null | undefined): string {
  return [title?.trim(), category?.trim()].filter(Boolean).join(' · ');
}

/** "1:05", "0:09"; "now" once the time is up. `goesAt` in unix seconds. */
export function raidLeft(goesAt: number, nowMs: number): string {
  const s = Math.ceil(goesAt - nowMs / 1000);
  if (!(s > 0)) return 'now';
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

/** "1 viewer", "1.234 viewers". */
export function viewersText(n: number, format: (n: number) => string): string {
  return `${format(n)} ${n === 1 ? 'viewer' : 'viewers'}`;
}

/** The pin's fold while raids come and go. `open`: unfolded. `before`: how it was when the
 * raid started (null when no raid shows). `hand`: folded or unfolded by hand during the raid,
 * so that choice stays. Hidden or closed pins are the event strip's and stay as they are. */
export interface PinFold {
  open: boolean;
  before: boolean | null;
  hand: boolean;
}

export const PIN_OPEN: PinFold = { open: true, before: null, hand: false };

/** A raid shows or stops showing. Same input twice changes nothing. */
export function raidFold(s: PinFold, raiding: boolean): PinFold {
  if (raiding && s.before === null) return { open: false, before: s.open, hand: false };
  if (!raiding && s.before !== null) return { open: s.hand ? s.open : s.before, before: null, hand: false };
  return s;
}

/** The pin is folded or unfolded by hand. */
export function handFold(s: PinFold): PinFold {
  return { open: !s.open, before: s.before, hand: s.before !== null };
}

/** A new pin starts open, folded right away while a raid shows. */
export function newPin(raiding: boolean): PinFold {
  return raidFold(PIN_OPEN, raiding);
}

/** Folded for the raid: the one-line form (no head, one line of text). */
export function raidSlim(s: PinFold): boolean {
  return s.before !== null && !s.hand && !s.open;
}

/** Raid pop-outs the app made (tab ids) that lost their window and were not docked: the tab
 * was only there for the pop-out, so it goes. `seen` remembers which were popped out once. */
export function raidTabsToClose(marked: readonly string[], popped: ReadonlySet<string>, seen: Set<string>): string[] {
  const out: string[] = [];
  for (const id of marked) {
    if (popped.has(id)) seen.add(id);
    else if (seen.has(id)) out.push(id);
  }
  for (const id of [...seen]) if (!marked.includes(id) || out.includes(id)) seen.delete(id);
  return out;
}
