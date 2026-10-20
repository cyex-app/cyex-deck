// t5 A page's Reset clears only what that page shows. One partition can feed several pages
// (general: General, Sound, Streamer mode; chat: Chat, Emotes and badges), and resetting the
// whole file also wiped the other pages, nicknames and the presence choice.

import type { Entry } from './registry/entry';

/** A merge patch that sets every key the page shows in `partition` back to its default
 * (`null` resets a key in Rust). Empty when the page shows none. */
export function pageResetPatch(entries: readonly Entry[], partition: string, page: string): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  const prefix = `${partition}.`;
  for (const e of entries) {
    if (e.page !== page) continue;
    for (const id of [e.id, ...(e.keys ?? [])]) {
      if (id.startsWith(prefix)) put(out, id.slice(prefix.length).split('.'));
    }
  }
  return out;
}

function put(obj: Record<string, unknown>, path: string[]): void {
  let at = obj;
  for (const key of path.slice(0, -1)) {
    const next = at[key];
    if (next === null) return; // A parent already resets as a whole.
    if (typeof next !== 'object') at[key] = {};
    at = at[key] as Record<string, unknown>;
  }
  at[path[path.length - 1]] = null;
}
