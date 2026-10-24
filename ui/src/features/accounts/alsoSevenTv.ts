// V108-7P "Also connect 7TV?": asked once when an account gets Sign in with a code plus Sign
// in with Twitch or Twitch pages (either order) and 7TV isn't connected. "Not now" is
// remembered per account on this PC. Pure apart from the storage calls.

import type { AccountView } from '../../ipc/bindings/AccountView';

export type Link = 'helix' | 'gql' | 'pages';

const KEY = 'cyex.sevenTvNotNow';

/** The account's Twitch connections, expired ones too; `null`: no such account yet. */
export function linksOf(a: Pick<AccountView, 'helix' | 'gql'> | undefined, pages: boolean): Link[] | null {
  if (!a) return null;
  const out: Link[] = [];
  if (a.helix) out.push('helix');
  if (a.gql) out.push('gql');
  if (pages) out.push('pages');
  return out;
}

/** The code sign-in together with Sign in with Twitch or Twitch pages. */
function paired(links: Link[]): boolean {
  return links.includes('gql') && (links.includes('helix') || links.includes('pages'));
}

/** Whether adding `added` to an account that had `before` makes the pair (a new account, a
 * sign-in again or a pair that was there already doesn't). */
export function asksSevenTv(added: Link, before: Link[] | null, sevenTvOn: boolean, notNow: boolean): boolean {
  if (!before || sevenTvOn || notNow || paired(before)) return false;
  return paired([...before, added]);
}

export function readNotNow(): string[] {
  try {
    const v: unknown = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

export function saveNotNow(ids: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(ids));
  } catch {
    // Not remembered; it may ask again after a restart.
  }
}
