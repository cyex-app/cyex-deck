// M12 Whether someone is a mod or VIP here already, so name menus offer Unmod and Remove VIP
// instead of Mod and VIP.

import type { MenuEntry } from '../../components/menuItems';

/** What someone already is here, as far as their badges or the viewer list tell. */
export interface Standing {
  mod: boolean;
  vip: boolean;
}

/** From a line's badges. Head mods count as mods. */
export function standingOf(badges: readonly { set: string }[] | undefined): Standing | undefined {
  if (!badges) return undefined;
  const has = (set: string) => badges.some((b) => b.set === set);
  return { mod: has('moderator') || has('lead_moderator'), vip: has('vip') };
}

/** A mod gets Unmod only (Twitch won't make a mod a VIP), a VIP gets Mod and Remove VIP,
 * anyone else or unknown gets Mod and VIP. */
export function roleItems(login: string, run: (line: string) => void, is?: Standing): MenuEntry[] {
  if (is?.mod) return [{ label: 'Unmod', icon: 'shield', run: () => run(`/unmod ${login}`) }];
  return [
    { label: 'Mod', icon: 'shield', run: () => run(`/mod ${login}`) },
    is?.vip
      ? { label: 'Remove VIP', icon: 'gem', run: () => run(`/unvip ${login}`) }
      : { label: 'VIP', icon: 'gem', run: () => run(`/vip ${login}`) },
  ];
}
