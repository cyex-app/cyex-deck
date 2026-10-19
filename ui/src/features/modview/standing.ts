// M12 Whether someone is a mod or VIP here already, so name menus and the usercard offer
// Unmod and Unvip instead of Mod and VIP. V107-M8 Live: a change that goes through (ours or
// another mod's) wins over badges until the badges catch up.

import type { MenuEntry } from '../../components/menuItems';
import type { RoleChange } from '../../ipc/bindings/RoleChange';
import { roleOf } from './roles.ts';

/** What someone already is here, as far as their badges or the viewer list tell. */
export interface Standing {
  mod: boolean;
  vip: boolean;
}

/** From a line's badges. Head mods count as mods. */
export function standingOf(badges: readonly { set: string }[] | undefined): Standing | undefined {
  if (!badges) return undefined;
  const role = roleOf(badges);
  return { mod: role === 'moderator' || role === 'leadModerator', vip: badges.some((b) => b.set === 'vip') };
}

/** V107-M8 What a change says about someone. Granting one role clears the other (Twitch
 * won't stack them; Rust takes the other away first). */
export function applyChange(prev: Partial<Standing> | undefined, c: Pick<RoleChange, 'moderator' | 'on'>): Partial<Standing> {
  const next = { ...prev, [c.moderator ? 'mod' : 'vip']: c.on };
  if (c.on) next[c.moderator ? 'vip' : 'mod'] = false;
  return next;
}

/** `seen` (badges or a lookup) with what changes since say on top; `undefined` when neither knows. */
export function merged(seen: Standing | undefined, changed: Partial<Standing> | undefined): Standing | undefined {
  if (!seen && !changed) return undefined;
  return { mod: false, vip: false, ...seen, ...changed };
}

/** Mod or Unmod, then VIP or Unvip. Granting one to someone who holds the other swaps them. */
export function roleItems(login: string, run: (line: string) => void, is?: Standing): MenuEntry[] {
  return [
    is?.mod
      ? { label: 'Unmod', icon: 'shield', run: () => run(`/unmod ${login}`) }
      : { label: 'Mod', icon: 'shield', run: () => run(`/mod ${login}`) },
    is?.vip
      ? { label: 'Unvip', icon: 'gem', run: () => run(`/unvip ${login}`) }
      : { label: 'VIP', icon: 'gem', run: () => run(`/vip ${login}`) },
  ];
}
