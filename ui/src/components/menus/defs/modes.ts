// V112-27 The chat modes menu: the mode chip in a chat's header, for mods and up. Rows show
// what ROOMSTATE says; slow mode and followers-only open a length dialog over the chat when off (V112-30).

import type { RoomState } from '../../../ipc/bindings/RoomState';
import type { LengthKind, ModeChange } from '../../../features/chat/roomModes.ts';
import { formatDuration } from '../../../lib/format.ts';
import { type MenuDef, SEP } from '../types.ts';
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
    row('emote', 'Emote only', 'smile', (c) => ({ label: 'Emote only', checked: c.state.emoteOnly, run: () => c.set({ kind: 'emote', on: !c.state.emoteOnly }) }), { renamable: false }),
    row('subs', 'Subscribers only', 'star', (c) => ({ label: 'Subscribers only', checked: c.state.subsOnly, run: () => c.set({ kind: 'subs', on: !c.state.subsOnly }) }), { renamable: false }),
    row('unique', 'Unique chat', 'copy', (c) => ({ label: 'Unique chat', checked: c.state.unique, run: () => c.set({ kind: 'unique', on: !c.state.unique }) }), { renamable: false }),
    row(
      'slow',
      'Slow mode',
      'clock',
      (c) =>
        c.state.slow
          ? { label: 'Slow mode', checked: true, hint: formatDuration(c.state.slow), run: () => c.set({ kind: 'slow', secs: null }) }
          : { label: 'Slow mode', run: () => c.ask('slow') },
      { renamable: false },
    ),
    row(
      'followers',
      'Followers only',
      'users',
      (c) => {
        const now = c.state.followersOnly;
        if (now === null) {
          return { label: 'Followers only', run: () => c.ask('followers') };
        }
        return {
          label: 'Followers only',
          checked: true,
          hint: now ? formatDuration(now * 60) : 'Any',
          run: () => c.set({ kind: 'followers', mins: null }),
        };
      },
      { renamable: false },
    ),
  ],
  defaults: [item('emote'), item('subs'), item('unique'), SEP, item('slow'), item('followers')],
};

export function modesSample(_who: Who): ModesCtx {
  return { state: { slow: 0, followersOnly: null, subsOnly: false, emoteOnly: false, unique: false }, set: noop, ask: noop };
}
