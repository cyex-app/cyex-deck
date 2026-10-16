// What a sign-in asks before it starts: tickboxes for 7TV and for subscriptions and wallet
// (Twitch's two sign-ins). Pure, so it's unit tested.

import type { AccountView } from '../../ipc/bindings/AccountView';
import type { SelectError } from '../../ipc/bindings/SelectError';
import type { TokenKind } from '../../ipc/bindings/TokenKind';
import { needsSignIn } from '../accounts/status.ts';

export type Part = 'seventv' | 'twitch';
/** Where the sign-in started: the 7TV menu (and a pick there or in chat), a subscription or
 * wallet prompt, or Settings → Accounts. */
export type Entry = 'seventv' | 'twitch' | 'settings';

export interface Done {
  seventv: boolean;
  twitch: boolean;
}

export interface Item {
  part: Part;
  label: string;
  /** Unlocked already: shown ticked and locked. */
  done: boolean;
  /** Ticked when the dialog opens. */
  ticked: boolean;
}

export type Plan =
  /** Nothing to do. */
  | { kind: 'none' }
  /** One thing left: no choice to make. `consent`: 7TV's yes is still needed first. */
  | { kind: 'run'; parts: Part[]; consent: boolean }
  | { kind: 'ask'; items: Item[] };

const LABEL: Record<Part, string> = {
  seventv: '7TV — your paint and badge',
  twitch: 'Subscriptions and wallet — subs, payment methods, the miner, channel points',
};

/** The part an entry is about (ticked first); Settings is about neither. */
const OWN: Record<Entry, Part | null> = { seventv: 'seventv', twitch: 'twitch', settings: null };

export function plan(entry: Entry, done: Done, consent: boolean): Plan {
  const left = (['seventv', 'twitch'] as const).filter((p) => !done[p]);
  const own = OWN[entry];
  if (left.length === 0 || (own && done[own])) return { kind: 'none' };
  if (left.length === 1) return { kind: 'run', parts: left, consent: left[0] === 'seventv' && !consent };
  return {
    kind: 'ask',
    items: (['seventv', 'twitch'] as const).map((part) => ({
      part,
      label: LABEL[part],
      done: done[part],
      ticked: done[part] || part === own,
    })),
  };
}

/** The parts ticked and not done yet: what Sign in runs. */
export function chosen(items: Item[], ticks: Record<Part, boolean>): Part[] {
  return items.filter((i) => !i.done && ticks[i.part]).map((i) => i.part);
}

/** Twitch's sign-ins this account still needs for subscriptions and wallet, in order. */
export function twitchLeft(a: Pick<AccountView, 'helix' | 'gql'>): TokenKind[] {
  const out: TokenKind[] = [];
  if (!a.helix || needsSignIn(a.helix)) out.push('helix');
  if (!a.gql || needsSignIn(a.gql)) out.push('gql');
  return out;
}

export function doneOf(a: Pick<AccountView, 'helix' | 'gql' | 'seventv'>): Done {
  return { seventv: a.seventv.consent && a.seventv.signedIn, twitch: twitchLeft(a).length === 0 };
}

/** 7TV's consent shows (and the sign-in click gives it) when 7TV is picked without one. */
export function asksConsent(parts: Part[], consentGiven: boolean): boolean {
  return parts.includes('seventv') && !consentGiven;
}

/** 7TV's window borrows the account's Twitch web profile only when Twitch is picked too, so
 * one Twitch login covers both. "Just 7TV" stays in a private window. */
export function sharedProfile(parts: Part[]): boolean {
  return parts.includes('seventv') && parts.includes('twitch');
}

/** Steps run Twitch first, so its login is there when 7TV's window opens in the same profile. */
export function order(parts: Part[]): Part[] {
  return (['twitch', 'seventv'] as const).filter((p) => parts.includes(p));
}

export const PART_NAME: Record<Part, string> = { seventv: '7TV', twitch: 'Subscriptions and wallet' };

/** "7TV is set. Subscriptions and wallet didn’t finish." for a run that stopped half way. */
export function leftText(done: Part[], failed: Part[]): string {
  const did = done.map((p) => PART_NAME[p]);
  const not = failed.map((p) => PART_NAME[p]).join(' and ');
  const lead = did.length ? `${did.join(' and ')} ${did.length > 1 ? 'are' : 'is'} set. ` : '';
  return `${lead}${not} didn’t finish. Try again any time.`;
}

/** A paint, badge or combo pick that needs a 7TV sign-in first: the pop-up asks for it.
 * Only with consent (without it, the menu shows the consent itself). */
export function pickNeedsSignIn(status: string, consent: boolean, error?: SelectError | null): boolean {
  if (!consent) return false;
  if (error) return error.type === 'signedOut' || error.type === 'signInAgain';
  return status === 'signedOut' || status === 'expired';
}
