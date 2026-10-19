// Rules for which users the looks store keeps (V107-M4). Pure, so it's testable without runes.

import type { Look } from '../../ipc/bindings/Look';

/** Most users kept; the oldest go first. Redrawn users just ask again. */
export const MAX_USERS = 4000;

/** A push (changes from any joined chat) only touches users already known to the UI. */
export function applyPushed(users: Map<string, Look | null>, incoming: Record<string, Look | null | undefined>) {
  for (const [id, look] of Object.entries(incoming)) {
    if (users.has(id)) users.set(id, look ?? null);
  }
}

/** An answer to our own ask: keep everything, then trim the oldest past the cap. */
export function applyAnswer(
  users: Map<string, Look | null>,
  incoming: Record<string, Look | null | undefined>,
  max = MAX_USERS,
) {
  for (const [id, look] of Object.entries(incoming)) users.set(id, look ?? null);
  trim(users, max);
}

export function trim(users: Map<string, Look | null>, max = MAX_USERS) {
  let extra = users.size - max;
  if (extra <= 0) return;
  for (const id of users.keys()) {
    users.delete(id);
    if (--extra <= 0) break;
  }
}
