// U4 K11 V106-31/32 The 7TV entries of an emote's right-click menu, fetched with a time limit.
// Shared by the chat and the emote picker.

import type { MenuEntry } from '../../../components/menuItems.ts';
import type { EditOptions } from '../../../ipc/bindings/EditOptions';
import { dismissWatch } from './dismiss.ts';
import { type EditHandlers, editEntries } from './menuItems.ts';
import type { editOptionsFor } from './options.ts';

/** The menu never waits longer than this for 7TV; without an answer it opens plain. */
export const EDIT_WAIT_MS = 500;

/** Only the newest right-click opens a menu; a slow one is dropped. */
let newest = 0;

/** A right-click that opens something else (a name, a message) drops a pending emote menu. */
export function cancelEmoteMenu(): void {
  newest++;
}

/** `o`, or null when it hasn't come within `waitMs`. */
export async function within(o: Promise<EditOptions | null>, waitMs: number): Promise<EditOptions | null> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    return await Promise.race([o, new Promise<null>((r) => (timer = setTimeout(() => r(null), waitMs)))]);
  } finally {
    clearTimeout(timer);
  }
}

export interface Deps {
  /** Where presses, Escape, the wheel and a lost window count as moving on. Default: the window. */
  target?: EventTarget;
  load?: typeof editOptionsFor;
}

/** The 7TV entries for one emote: [] when there are none to offer, null when the menu shouldn't
 *  open any more (the user moved on, or a newer right-click came in). */
export async function emoteEditMenu(
  account: string | null,
  login: string,
  emote: { id: string; name: string; provider: string },
  handlers: EditHandlers,
  waitMs = EDIT_WAIT_MS,
  deps: Deps = {},
): Promise<MenuEntry[] | null> {
  const ask = ++newest;
  if (!account || emote.provider !== 'seventv') return [];
  const stop = dismissWatch(deps.target ?? window);
  // Loaded on use so the pure parts above test without the IPC layer.
  const load = deps.load ?? (await import('./options.ts')).editOptionsFor;
  const o = await within(load(account, login, emote.id, emote.name), waitMs);
  const gone = stop();
  if (gone || ask !== newest) return null;
  return editEntries(o, handlers);
}
