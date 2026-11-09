// V101-17 The menu every player shares, from the gear and from a right-click: one order and
// wording. V110-U66 The rows and their built-in order live in the menu registry
// (components/menus/defs/player.ts); `playerMenu` is that built-in menu, `playerEntries`
// (entries.ts) the one with the user's layout.

import type { MenuEntry } from '../../components/menu.svelte';
import { PLAYER_DEF, type PlayerMenu } from '../../components/menus/defs/player.ts';
import { resolveMenu } from '../../components/menus/resolve.ts';
import type { MenuLayout } from '../../components/menus/types.ts';

export { type MomentRow, type PlayerMenu, SPEEDS } from '../../components/menus/defs/player.ts';

export function playerMenu(m: PlayerMenu, layout?: MenuLayout | null): MenuEntry[] {
  return resolveMenu(PLAYER_DEF, layout, m);
}
