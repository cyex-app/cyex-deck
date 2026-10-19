// V107 Who may delete which chat message, as Twitch has it: Helix "Delete Chat Messages"
// (the message is under 6 hours old, not the broadcaster's, not another moderator's) and
// what GQL did on 2026/10/19 (the broadcaster deleted their own line). The UI greys the
// button out with the reason; Twitch still has the last word.

import type { ModRole } from '../../ipc/bindings/ModRole';

/** Twitch deletes messages up to 6 hours old. */
export const DELETE_WINDOW_MS = 6 * 60 * 60 * 1000;

export interface DeleteTarget {
  /** The sender's user id and login. */
  id: string;
  login: string;
  /** Their role here, from their badges; `null` = none or not known. */
  role: ModRole | null;
  /** When it was sent (ms), if known. */
  ts?: number | null;
}

/** `null` when `actor` may delete the message, else why not (short, for a tooltip).
 * `me` is the acting account's user id, `channel` the channel's login. */
export function cantDelete(
  actor: ModRole | null | undefined,
  me: string | null,
  target: DeleteTarget,
  channel: string,
  now: number,
): string | null {
  if (!actor) return 'Only mods can delete messages.';
  if (target.ts != null && now - target.ts > DELETE_WINDOW_MS) return 'Twitch only deletes messages from the last 6 hours.';
  if (actor === 'broadcaster') return null;
  if (target.role === 'broadcaster' || target.login.toLowerCase() === channel.toLowerCase())
    return 'Mods can’t delete the broadcaster’s messages.';
  if (target.role && target.id !== me) return 'Mods can’t delete other mods’ messages.';
  return null;
}
