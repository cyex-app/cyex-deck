// V101-33/34 What a mod gets on chat lines: the pin button, and in mod mode the buttons
// picked in Settings → Chat (M10), in the order dragged there (V105-15), in front (1.57:
// only where the account moderates).

import type { ModButton } from '../../ipc/bindings/ModButton';
import type { ModButtons } from '../../ipc/bindings/ModButtons';
import { roleOf } from './roles.ts';

export interface LineMod {
  /** The account's user id: its own lines get no buttons. */
  me: string | null;
  broadcaster: boolean;
  /** Mod mode is on in this chat. */
  inline: boolean;
  /** M10, V105-15 The buttons mod mode shows, in order. */
  slots: ModSlot[];
}

/** V105-15 A stable key for an order entry: `delete`, `warn`, `ban`, `t60`. */
export function buttonKey(b: ModButton): string {
  return typeof b === 'string' ? b : `t${b.timeout}`;
}

/** V105-15 One button in front of a line, in the user's order. */
export type ModSlot =
  | { key: string; kind: 'delete' | 'warn' | 'ban' }
  /** `short`: several timeouts show their lengths instead of the clock (M10). */
  | { key: string; kind: 'timeout'; secs: number; short: boolean };

/** V105-15 The saved order, with anything it lacks added last in the default order (as
 *  Rust does), and timeouts no longer picked left out. */
export function fullOrder(b: ModButtons): ModButton[] {
  const defaults: ModButton[] = ['delete', ...b.timeouts.map((timeout) => ({ timeout })), 'warn', 'ban'];
  const out: ModButton[] = [];
  for (const x of [...b.order, ...defaults]) {
    if (typeof x !== 'string' && !b.timeouts.includes(x.timeout)) continue;
    if (!out.some((y) => buttonKey(y) === buttonKey(x))) out.push(x);
  }
  return out;
}

/** V105-15 The buttons that are on, in the order picked in Settings. */
export function modSlots(b: ModButtons): ModSlot[] {
  const short = b.timeouts.length > 1;
  const out: ModSlot[] = [];
  for (const x of fullOrder(b)) {
    if (typeof x !== 'string') out.push({ key: buttonKey(x), kind: 'timeout', secs: x.timeout, short });
    else if (b[x]) out.push({ key: x, kind: x });
  }
  return out;
}

interface Who {
  id: string;
  login: string;
  badges: { set: string }[];
}

/** Mod mode's buttons go on chat lines from people a mod can act on: not the broadcaster,
 * not the account itself, and not other mods unless the account is the broadcaster. */
export function modButtonsFor(mod: LineMod | null, sender: Who | null | undefined, channel: string): boolean {
  if (!mod?.inline || !sender || sender.id === mod.me || sender.login === channel) return false;
  if (!mod.slots.length) return false;
  return mod.broadcaster || roleOf(sender.badges) === null;
}
