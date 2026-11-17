// V102-20 Tools an account can't use are greyed out, not hidden, with one flag saying
// which sign-in unlocks them. Pure parts, unit tested.

import type { AccountView } from '../ipc/bindings/AccountView';
import type { TokenKind } from '../ipc/bindings/TokenKind';

/** The flag's words. V111-47 The two sign-ins go by two names only: `helix` is the Twitch
 * sign-in, `gql` is Extra Twitch features. */
export const FLAGS: Record<TokenKind, { label: string; hint: string }> = {
  gql: { label: 'Extra Twitch features', hint: 'Needs Extra Twitch features. Click to add them.' },
  helix: { label: 'Twitch sign-in', hint: 'Needs the Twitch sign-in. Click to add it.' },
};

/** V111-47 What Rust says when a sign-in is missing (the error contract), word for word. */
export const NEEDS_TEXT: Record<TokenKind, string> = {
  helix: 'Needs the Twitch sign-in.',
  gql: 'Needs Extra Twitch features.',
};

/** V111-47 Which sign-in an error text says is missing, or `null` for any other error. */
export function needsKind(text: string): TokenKind | null {
  const t = text.trim();
  if (t === NEEDS_TEXT.helix) return 'helix';
  if (t === NEEDS_TEXT.gql) return 'gql';
  return null;
}

/** V111-47 The account has a Twitch sign-in that works (no token, or one that ran out, doesn't). */
export function helixUsable(account: Pick<AccountView, 'helix'> | undefined): boolean {
  const s = account?.helix?.state;
  return s === 'checking' || s === 'valid' || s === 'offline';
}

/** What a greyed-out tool needs, or `null` when it's usable. */
export function lockFor(can: boolean, kind: TokenKind): TokenKind | null {
  return can ? null : kind;
}

/** V111-47 A tool for an account: usable, locked behind a sign-in it lacks, or no account at all
 * (nothing to unlock). */
export function access(signedIn: boolean, can: boolean): 'open' | 'locked' | 'none' {
  if (!signedIn) return 'none';
  return can ? 'open' : 'locked';
}

/** V108-N01 The account has a GQL token worth using (the same states Rust's `usable()` takes). */
export function gqlUsable(account: Pick<AccountView, 'gql'> | undefined): boolean {
  const s = account?.gql?.state;
  return s === 'checking' || s === 'valid' || s === 'offline';
}

/** Where a flag click lands in Settings → Accounts. V111-47 With an account, that account's
 * own card (its Twitch sign-in row, or its Extra Twitch features options); with none (nobody
 * is signed in yet), the matching way to add one. */
export function signInTarget(kind: TokenKind, account: AccountView | undefined): 'account' | 'twitch' | 'code' {
  if (account) return 'account';
  return kind === 'helix' ? 'twitch' : 'code';
}

/** V111-47 Which sign-in to offer an account that can't chat or clip: the one it has but that
 * ran out first, else the one it lacks (an account with a Twitch sign-in and no scope gets the
 * extra features, which do both too). */
export function missingSignIn(account: Pick<AccountView, 'helix' | 'gql'>): TokenKind {
  if (account.gql && !gqlUsable(account)) return 'gql';
  if (account.helix && !helixUsable(account)) return 'helix';
  return account.gql ? 'helix' : 'gql';
}
