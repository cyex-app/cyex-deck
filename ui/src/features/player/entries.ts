// V110-U66 The player menu with the user's layout (Settings › Menus).

import type { MenuEntry } from '../../components/menu.svelte';
import { PLAYER_DEF, type PlayerMenu } from '../../components/menus/defs/player';
import { menuEntries } from '../../components/menus/use';

export function playerEntries(m: PlayerMenu, chan = ''): MenuEntry[] {
  return menuEntries(PLAYER_DEF, m, { values: { chan } });
}
