// V109-C03 What the clips manager shows, kept out of the component for tests.

import type { Clip } from '../../ipc/bindings/Clip';
import type { ClipsOf } from '../../ipc/bindings/ClipsOf';

export const WHOSE = [
  { value: 'channel', label: 'My channel' },
  { value: 'mine', label: 'Made by me' },
] as const;

export const ORDERS = [
  { value: 'views', label: 'Top' },
  { value: 'newest', label: 'New' },
  { value: 'oldest', label: 'Old' },
] as const;

/** Who made it is the account: Twitch's editor is for the clip's maker. A name match is
 * the best the list gives (it carries the clipper's display name, not an id). */
export function madeBy(c: Clip, of: ClipsOf, names: string[]): boolean {
  if (of === 'mine') return true;
  const who = c.clippedBy?.toLowerCase();
  return !!who && names.some((n) => n.toLowerCase() === who);
}

/** The empty list's line. */
export function emptyLine(of: ClipsOf, searched: boolean): string {
  if (searched) return 'No clips match that.';
  return of === 'mine' ? 'You haven’t made clips in this time.' : 'No clips of your channel in this time.';
}
