// V103-13 The settings registry: every setting you can find in Settings, with its page,
// name, hint and other words for it. A new setting gets an entry in its group's file
// (`app.ts`, `account.ts`, `chat.ts`, `watching.ts`, `streaming.ts`) and `setting="<id>"`
// on its row; `registry.test.ts` fails until both are there.

import { ACCOUNT } from './account.ts';
import { APP } from './app.ts';
import { CHAT } from './chat.ts';
import type { Entry } from './entry';
import { STREAMING } from './streaming.ts';
import { WATCHING } from './watching.ts';

export type { Entry } from './entry';
export { GROUPS, PAGES, pageOf, type Page, type PageId } from './pages.ts';

export const ENTRIES: readonly Entry[] = [...APP, ...ACCOUNT, ...CHAT, ...WATCHING, ...STREAMING];

/** S2 Each hotkey is a result of its own, read from the hotkey list so new ones show up
 * without an entry. Rows on the Hotkeys page carry `hotkey:<action>`. */
export function hotkeyEntries(actions: Record<string, { label: string; default: string; global?: boolean }>): Entry[] {
  return Object.entries(actions).map(([id, a]) => ({
    id: `hotkey:${id}`,
    page: 'hotkeys',
    label: a.label,
    description: a.global ? 'Works while another app is in front.' : a.default ? `${a.default} unless you change it.` : undefined,
    keywords: ['hotkey', 'shortcut', 'keys'],
    anchor: 'hotkeys.bindings',
  }));
}
