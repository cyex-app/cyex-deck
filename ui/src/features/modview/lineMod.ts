// V101-33/34 What a mod gets on chat lines: the pin button, and in mod mode the buttons
// picked in Settings → Chat (M10), in the order dragged there (V105-15), in front (1.57:
// only where the account moderates).

import type { ModButton } from '../../ipc/bindings/ModButton';
import type { ModButtons } from '../../ipc/bindings/ModButtons';
import type { Caps } from '../../ipc/bindings/Caps';
import { may, whose } from './may.ts';

export interface LineMod {
  /** The account's user id: its own lines get no buttons. */
  me: string | null;
  /** V107-R14 What this account may do here (Rust's answer). */
  caps: Caps;
  /** Mod mode is on in this chat. */
  inline: boolean;
  /** M10, V105-15 The buttons mod mode shows, in order. */
  slots: ModSlot[];
}

/** V105-15 A stable key for an order entry: `delete`, `warn`, `ban`, `unban`, `t60`. */
export function buttonKey(b: ModButton): string {
  return typeof b === 'string' ? b : `t${b.timeout}`;
}

/** V105-15 One button in front of a line, in the user's order. */
export type ModSlot =
  | { key: string; kind: 'delete' | 'warn' | 'ban' | 'unban' }
  /** `short`: several timeouts show their lengths instead of the clock (M10). */
  | { key: string; kind: 'timeout'; secs: number; short: boolean };

/** V105-15 The saved order, with anything it lacks added last in the default order (as
 *  Rust does). Timeouts no longer picked stay in their place, shown off; `presets` are the
 *  timeouts that can be picked, so each gets a row even when the order never saw it. */
export function fullOrder(b: ModButtons, presets: readonly number[] = []): ModButton[] {
  const defaults: ModButton[] = ['delete', ...b.timeouts.map((timeout) => ({ timeout })), 'warn', 'ban', 'unban'];
  // Presets that aren't picked get a row too (shown off), after everything else.
  for (const timeout of presets) defaults.push({ timeout });
  const out: ModButton[] = [];
  for (const x of [...b.order, ...defaults]) {
    if (!out.some((y) => buttonKey(y) === buttonKey(x))) out.push(x);
  }
  return out;
}

/** V105-15 The buttons that are on, in the order picked in Settings. */
export function modSlots(b: ModButtons): ModSlot[] {
  const short = b.timeouts.length > 1;
  const out: ModSlot[] = [];
  for (const x of fullOrder(b)) {
    if (typeof x !== 'string') {
      if (b.timeouts.includes(x.timeout)) out.push({ key: buttonKey(x), kind: 'timeout', secs: x.timeout, short });
    } else if (b[x]) out.push({ key: x, kind: x });
  }
  return out;
}

/** Whether the button is on: a timeout when it is in the picked list, the others by flag. */
export function buttonOn(b: ModButtons, x: ModButton): boolean {
  return typeof x === 'string' ? b[x] : b.timeouts.includes(x.timeout);
}

/** M10 Four timeouts at most. */
export const MAX_TIMEOUTS = 4;

/** V105-15 Whether the order list can add this button: only a timeout can't, with 4 on. */
export function canAddButton(b: ModButtons, x: ModButton): boolean {
  return typeof x === 'string' || b.timeouts.includes(x.timeout) || b.timeouts.length < MAX_TIMEOUTS;
}

/** V105-15 The settings patch that adds a button the order list shows as off, or removes one
 *  that is on. No button is locked: warn, delete, ban, unban and every timeout flip alike; a fifth
 *  timeout can't be added. The order is kept, so a button comes back in its place. */
export function toggleButton(b: ModButtons, x: ModButton): Partial<ModButtons> {
  if (typeof x !== 'string') {
    const on = b.timeouts.includes(x.timeout);
    if (!on && !canAddButton(b, x)) return {};
    return { timeouts: on ? b.timeouts.filter((s) => s !== x.timeout) : [...b.timeouts, x.timeout] };
  }
  return { [x]: !b[x] };
}

/** V107-R22 The patch that adds a typed timeout as a button, on and last; or why not. */
export function addTimeout(b: ModButtons, secs: number): Partial<ModButtons> | string {
  if (b.timeouts.includes(secs)) return 'Already a button.';
  if (b.timeouts.length >= MAX_TIMEOUTS) return `${MAX_TIMEOUTS} timeouts max.`;
  const order = fullOrder(b).filter((x) => buttonKey(x) !== `t${secs}`);
  return { timeouts: [...b.timeouts, secs], order: [...order, { timeout: secs }] };
}

/** V107-R22 The patch that gives a timeout button another length, in its place and on or
 *  off as it was; or why not. */
export function retimeButton(b: ModButtons, from: number, to: number, presets: readonly number[] = []): Partial<ModButtons> | string {
  if (from === to) return {};
  if (b.timeouts.includes(to) || fullOrder(b, presets).some((x) => buttonKey(x) === `t${to}`)) return 'Already a button.';
  const order = fullOrder(b, presets).map((x) => (typeof x !== 'string' && x.timeout === from ? { timeout: to } : x));
  const timeouts = b.timeouts.map((s) => (s === from ? to : s));
  return { timeouts, order };
}

interface Who {
  id: string;
  login: string;
  badges: { set: string }[];
}

/** Mod mode's buttons go on chat lines from people a mod can act on: not the broadcaster,
 * not the account itself, and not other mods unless the account is the broadcaster or a
 * lead mod (V107-R14 Rust's `quick` answer; V107-R22b lead mods too). */
export function modButtonsFor(mod: LineMod | null, sender: Who | null | undefined, channel: string): boolean {
  if (!mod?.inline || !sender || !mod.slots.length) return false;
  return may(mod.caps, 'quick', whose(sender, mod.me, channel)).offered;
}

/** V107-R22b The buttons a line shows: a deleted line (its sender timed out or banned) keeps
 *  only Unban, so a ban can be lifted from their older lines. */
export function lineSlots(slots: ModSlot[], deleted: boolean): ModSlot[] {
  return deleted ? slots.filter((s) => s.kind === 'unban') : slots;
}
