// U4 K11 V106-31/32 The 7TV entries of an emote's right-click menu, fetched with a time limit.
// Shared by the chat and the emote picker.

import type { MenuEntry } from '../../../components/menuItems.ts';
import type { CyextvMenuOptions } from '../../../ipc/bindings/CyextvMenuOptions';
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
export async function within<T>(o: Promise<T | null>, waitMs: number): Promise<T | null> {
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
  /** V112-17 cyex.tv's side: what is offered, and the add. Default: the IPC. */
  tv?: {
    options: (account: string, login: string, emoteId: string) => Promise<CyextvMenuOptions>;
    add: (account: string, login: string, emoteId: string) => Promise<string>;
    said: (line: string, ok: boolean) => void;
  };
}

/** V112-17 "Add to cyex.tv" for a cyex.tv emote: nothing unless the account may edit this channel's
 *  active set; greyed with the reason when it is already there or the set is full. */
export function cyextvEntries(o: CyextvMenuOptions | null, add: () => void): MenuEntry[] {
  if (!o?.canEdit) return [];
  return ['separator', { label: 'Add to cyex.tv', icon: 'plus', disabled: !!o.note, note: o.note ?? undefined, run: add }];
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
  if (!account || (emote.provider !== 'seventv' && emote.provider !== 'cyextv')) return [];
  const stop = dismissWatch(deps.target ?? window);
  if (emote.provider === 'cyextv') {
    const tv = deps.tv ?? (await tvDeps());
    const o = await within(tv.options(account, login, emote.id).catch(() => null), waitMs);
    const gone = stop();
    if (gone || ask !== newest) return null;
    return cyextvEntries(o, () =>
      void tv.add(account, login, emote.id).then(
        (line) => tv.said(line, true),
        (e: unknown) => tv.said(typeof e === 'string' ? e : 'Couldn’t add it.', false),
      ),
    );
  }
  // Loaded on use so the pure parts above test without the IPC layer.
  const load = deps.load ?? (await import('./options.ts')).editOptionsFor;
  const o = await within(load(account, login, emote.id, emote.name), waitMs);
  const gone = stop();
  if (gone || ask !== newest) return null;
  return editEntries(o, handlers);
}

/** The IPC and the toast, loaded on use. */
async function tvDeps(): Promise<NonNullable<Deps['tv']>> {
  const [{ cyextvIpc }, { toast }] = await Promise.all([import('../../../ipc/cyextv.ts'), import('../../../lib/toast.svelte.ts')]);
  return {
    options: cyextvIpc.emoteOptions,
    add: cyextvIpc.emoteAdd,
    said: (line, ok) => (ok ? toast.success(line) : toast.error(line)),
  };
}
