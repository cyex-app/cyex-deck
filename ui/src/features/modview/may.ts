// V107-R14 Who may do what, as Rust answers it (crates/moderation/src/may.rs). The answer
// comes with the role (`RoleEntry.may`), so menus stay synchronous: this only sorts a line
// into a `Whose` and picks the cell.

import type { Caps } from '../../ipc/bindings/Caps';
import type { Cell } from '../../ipc/bindings/Cell';
import { roleOf } from './roles.ts';

export type Act = 'user' | 'pin' | 'delete' | 'quick';
export type Whose = 'anyone' | 'own' | 'mod' | 'broadcaster';

export interface Sender {
  /** User id; empty when not known. */
  id: string;
  login: string;
  badges?: readonly { set: string }[];
}

/** What `s` is to the account `me` in `channel`: the channel itself (by login or badge),
 * the account's own, a mod, anyone else. */
export function whose(s: Sender, me: string | null, channel: string): Whose {
  const role = roleOf(s.badges);
  if (s.login.toLowerCase() === channel.toLowerCase() || role === 'broadcaster') return 'broadcaster';
  if (s.id && s.id === me) return 'own';
  return role ? 'mod' : 'anyone';
}

const NOTHING: Cell = { offered: false, why: null };

/** The answer for `act` on a line sent at `ts` (ms; unknown counts as fresh). No caps (not
 * a mod here): nothing is offered. */
export function may(caps: Caps | null | undefined, act: Act, w: Whose, ts?: number | null, now = Date.now()): Cell {
  if (!caps) return NOTHING;
  const old = ts != null && now - ts > caps.windowSecs * 1000;
  const age = caps[act][w];
  return old ? age.old : age.fresh;
}
