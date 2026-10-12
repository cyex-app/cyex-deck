// Wording for account and token states. Pure, so it's unit tested.

import type { AccountView } from '../../ipc/bindings/AccountView';
import type { TokenView } from '../../ipc/bindings/TokenView';
import { formatDate, formatTime } from '../../lib/format.ts';

export type Tone = 'ok' | 'warn' | 'bad' | 'muted';

export function tokenStatus(t: TokenView): { text: string; tone: Tone } {
  switch (t.state) {
    case 'invalid':
      return { text: 'Expired or revoked', tone: 'bad' };
    case 'missing':
      return { text: 'Not saved on this PC', tone: 'bad' };
    case 'offline':
      return { text: "Couldn't reach Twitch, trying again soon", tone: 'warn' };
    case 'checking':
      return { text: 'Checking…', tone: 'muted' };
  }
  if (t.expiresAt === null) return { text: "Signed in, doesn't expire", tone: 'ok' };
  // Device sign-ins renew on their own, so their expiry isn't worth showing.
  if (t.source === 'device') return { text: 'Signed in', tone: 'ok' };
  const d = new Date(t.expiresAt * 1000);
  return { text: `Works until ${formatDate(d)} ${formatTime(d)}`, tone: 'ok' };
}

export function needsSignIn(t: TokenView | null): boolean {
  return t?.state === 'invalid' || t?.state === 'missing';
}

export function accountNeedsAttention(a: AccountView): boolean {
  return needsSignIn(a.helix) || needsSignIn(a.gql);
}

/** "Display (login)" when the display name isn't just the login, e.g. localized names. */
export function accountName(a: AccountView): string {
  return a.displayName.toLowerCase() === a.login ? a.displayName : `${a.displayName} (${a.login})`;
}

export function missingScopesText(count: number): string {
  return `${count} permission${count === 1 ? '' : 's'} missing, so some tools are hidden. Sign in again to fix it.`;
}
