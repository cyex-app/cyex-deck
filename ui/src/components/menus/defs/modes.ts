// V112-27 The chat modes menu: the mode chip in a chat's header, for mods and up. Rows show
// what ROOMSTATE says; slow mode and followers-only open a length dialog over the chat when off (V112-30).

import type { RoomState } from '../../../ipc/bindings/RoomState';
import type { LengthKind, ModeChange } from '../../../features/chat/roomModes.ts';
import { formatDuration } from '../../../lib/format.ts';
import type { MenuDef } from '../types.ts';
import type { MenuItem } from '../../menuItems.ts';
import { item } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface ModesCtx {
  state: Pick<RoomState, 'slow' | 'followersOnly' | 'subsOnly' | 'emoteOnly' | 'unique'>;
  set(change: ModeChange): void;
  /** Slow mode and followers-only are off: opens the dialog that asks for the length. */
  ask(kind: LengthKind): void;
}

export const MODES_DEF: MenuDef<ModesCtx> = {
  id: 'chat.modes',
  label: 'Chat modes menu',
  where: 'The mode chip in a chat’s header (mods)',
  items: [
    // V112-31 (user) Checkboxes that keep the menu open, so several modes can be set in a row.
    // Only slow and followers-only, when off, close it for their length dialog.
    row('emote', 'Emote only', 'smile', (c) => toggle('Emote only', c.state.emoteOnly, (on) => c.set({ kind: 'emote', on })), { renamable: false }),
    row('subs', 'Subscribers only', 'star', (c) => toggle('Subscribers only', c.state.subsOnly, (on) => c.set({ kind: 'subs', on })), { renamable: false }),
    row('unique', 'Unique chat', 'copy', (c) => toggle('Unique chat', c.state.unique, (on) => c.set({ kind: 'unique', on })), { renamable: false }),
    row(
      'slow',
      'Slow mode',
      'clock',
      (c) =>
        lengthRow('Slow mode', c.state.slow ? formatDuration(c.state.slow) : null, () => c.set({ kind: 'slow', secs: null }), () => c.ask('slow')),
      { renamable: false },
    ),
    row(
      'followers',
      'Followers only',
      'users',
      (c) => {
        const now = c.state.followersOnly;
        const hint = now === null ? null : now ? formatDuration(now * 60) : 'Any';
        return lengthRow('Followers only', hint, () => c.set({ kind: 'followers', mins: null }), () => c.ask('followers'));
      },
      { renamable: false },
    ),
  ],
  defaults: [item('emote'), item('subs'), item('unique'), item('slow'), item('followers')],
};

/** A mode that is only on or off. The tick and the next change follow each pick. */
function toggle(label: string, on: boolean, set: (on: boolean) => void): MenuItem {
  let now = on;
  return {
    label,
    checked: on,
    stay: true,
    run: () => {
      now = !now;
      set(now);
    },
  };
}

/** Slow mode and followers-only: on, a pick turns it off and the menu stays; off, a pick
 * asks for the length (the dialog closes the menu). `hint` is the length while on, else null. */
function lengthRow(label: string, hint: string | null, off: () => void, ask: () => void): MenuItem {
  // `ask` closes the menu itself (the dialog takes over); `stay` only draws the box here.
  if (hint === null) return { label, checked: false, stay: true, run: ask };
  let on = true;
  return {
    label,
    checked: true,
    hint,
    stay: true,
    run: () => {
      if (on) {
        on = false;
        off();
      } else {
        ask();
      }
    },
  };
}

export function modesSample(_who: Who): ModesCtx {
  return { state: { slow: 0, followersOnly: null, subsOnly: false, emoteOnly: false, unique: false }, set: noop, ask: noop };
}
