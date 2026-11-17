// Words for subs (1.28): tier names and where a sub stands.

import { formatDate } from '../../lib/format.ts';
import type { MySub } from '../../ipc/bindings/MySub';

export function tierLabel(tier: string): string {
  const n = Number(tier) / 1000;
  return Number.isInteger(n) && n >= 1 ? `Tier ${n}` : 'Sub';
}

const day = (iso: string | null) => (iso ? formatDate(new Date(iso)) : '');

/** "Renews 2026/11/01", "Ends 2026/11/01", "Gift from x", "Prime". */
export function standing(s: Pick<MySub, 'renewsAt' | 'endsAt' | 'prime' | 'gifter' | 'canceled'>): string {
  if (s.gifter) return s.endsAt ? `Gift from ${s.gifter}, ends ${day(s.endsAt)}` : `Gift from ${s.gifter}`;
  if (s.prime) return s.endsAt ? `Prime, ends ${day(s.endsAt)}` : 'Prime';
  if (s.canceled || !s.renewsAt) return s.endsAt ? `Ends ${day(s.endsAt)}` : 'Doesn’t renew';
  return `Renews ${day(s.renewsAt)}`;
}

/** Renewal can be turned off here: a paid web sub that renews. */
export function canStop(s: Pick<MySub, 'renewsAt' | 'prime' | 'gifter' | 'canceled' | 'platform'>): boolean {
  return !s.canceled && !s.prime && !s.gifter && !!s.renewsAt && (s.platform === 'WEB' || s.platform === '');
}

export function months(n: number | null): string {
  return n ? `${n} month${n === 1 ? '' : 's'}` : '';
}

/** V111-52 What the tab shows. The list is GQL data, so GQL alone opens it; Twitch's own
 *  sign-in only adds the pages that open on twitch.tv (payment methods, manage, resub). */
export type SubsGate = 'signedOut' | 'needsGql' | 'list';

export function subsGate(account: string | null, gql: boolean): SubsGate {
  if (account === null) return 'signedOut';
  return gql ? 'list' : 'needsGql';
}
