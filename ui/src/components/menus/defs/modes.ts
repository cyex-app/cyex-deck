// V112-27 The chat modes menu: the mode chip in a chat's header, for mods and up. Rows show
// what ROOMSTATE says; slow mode and followers-only open a typed length when off (V112-29).

import type { RoomState } from '../../../ipc/bindings/RoomState';
import { followText, MODE_DEFAULTS, type ModeChange, parseFollowers, parseSlow, slowText } from '../../../features/chat/roomModes.ts';
import { formatDuration } from '../../../lib/format.ts';
import { type MenuDef, SEP } from '../types.ts';
import { item } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface ModesCtx {
  state: Pick<RoomState, 'slow' | 'followersOnly' | 'subsOnly' | 'emoteOnly' | 'unique'>;
  set(change: ModeChange): void;
  /** The lengths last applied, for the field's start value. */
  last?: { slow: number; followers: number };
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
          : {
              label: 'Slow mode',
              items: [
                {
                  label: 'Slow mode length',
                  field: {
                    placeholder: 'e.g. 30s or 2m',
                    value: slowText((c.last ?? MODE_DEFAULTS).slow),
                    focus: true,
                    submit: (text) => {
                      const p = parseSlow(text);
                      if ('error' in p) return p.error;
                      c.set({ kind: 'slow', secs: p.value });
                      return null;
                    },
                  },
                },
              ],
            },
      { renamable: false },
    ),
    row(
      'followers',
      'Followers only',
      'users',
      (c) => {
        const now = c.state.followersOnly;
        if (now === null) {
          return {
            label: 'Followers only',
            items: [
              {
                label: 'Followers only length',
                field: {
                  placeholder: 'e.g. 10m or 1d',
                  value: followText((c.last ?? MODE_DEFAULTS).followers),
                  focus: true,
                  submit: (text) => {
                    const p = parseFollowers(text);
                    if ('error' in p) return p.error;
                    c.set({ kind: 'followers', mins: p.value });
                    return null;
                  },
                },
              },
            ],
          };
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
  return { state: { slow: 0, followersOnly: null, subsOnly: false, emoteOnly: false, unique: false }, set: noop };
}
