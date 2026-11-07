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

/** V109-C05 Longest clip title (Twitch’s field: 100 UTF-16 units, as `maxlength` counts). */
export const TITLE_MAX = 100;

/** V109-C05 Rename only where Twitch’s clips manager offers Edit: any clip of the account’s
 * own channel, and clips it made elsewhere unless Twitch says it may no longer
 * (`editRestricted`, unknown counts as no). */
export function canRename(c: Clip, of: ClipsOf, login: string | undefined): boolean {
  if (of === 'channel') return true;
  if (login && c.channelLogin.toLowerCase() === login.toLowerCase()) return true;
  return c.editRestricted === false;
}

/** V109-C05 A typed title, or why it can’t be sent; null: nothing changed. */
export function titleCheck(text: string, was: string): { title: string } | { error: string } | null {
  const t = text.trim();
  if (!t) return { error: 'Add a title.' };
  if (t.length > TITLE_MAX) return { error: `Titles can be ${TITLE_MAX} characters at most.` };
  return t === was.trim() ? null : { title: t };
}

/** The empty list's line. */
export function emptyLine(of: ClipsOf, searched: boolean): string {
  if (searched) return 'No clips match that.';
  return of === 'mine' ? 'You haven’t made clips in this time.' : 'No clips of your channel in this time.';
}
