// V102-20 Tools an account can't use are greyed out, not hidden, with one flag saying
// which sign-in unlocks them. Pure parts, unit tested.

import type { AccountView } from '../ipc/bindings/AccountView';
import type { TokenKind } from '../ipc/bindings/TokenKind';

/** The flag's words: `helix` is Sign in with Twitch. */
export const FLAGS: Record<TokenKind, { label: string; hint: string }> = {
  gql: { label: 'GQL only', hint: 'Needs a GQL sign-in. Click to add one.' },
  helix: { label: 'Twitch sign-in', hint: 'Needs Sign in with Twitch. Click to sign in.' },
};

/** What a greyed-out tool needs, or `null` when it's usable. */
export function lockFor(can: boolean, kind: TokenKind): TokenKind | null {
  return can ? null : kind;
}

/** Where a flag click lands in Settings → Accounts. GQL for an account signed in with
 * Twitch opens that account's token and code options; otherwise the matching way to
 * sign in. */
export function signInTarget(kind: TokenKind, current: AccountView | undefined): 'account' | 'twitch' | 'code' {
  if (kind === 'helix') return 'twitch';
  return current?.helix && !current.gql ? 'account' : 'code';
}
