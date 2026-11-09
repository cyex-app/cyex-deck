// V110-U66 Right-click an emote in chat. The 7TV edits come from 7TV and arrive as one slot.

import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type MenuDef } from '../types.ts';
import { noop, row, slot, type Who } from './make.ts';

export interface EmoteCtx {
  name: string;
  /** An unlisted 7TV emote: shows the blur switch. */
  unlisted: boolean;
  /** Unlisted emotes are blurred now. */
  blurred: boolean;
  /** In the account's favorites. */
  favorite: boolean;
  /** V106-31/32 The 7TV edits (add, rename, remove): empty without a 7TV sign-in or an editable channel. */
  edits: MenuEntry[];
  act: { card(): void; copyName(): void; hide(): void; blur(): void; favorite(): void; copyLink(): void };
}

export const EMOTE_DEF: MenuDef<EmoteCtx> = {
  id: 'chat.emote',
  label: 'Emotes',
  where: 'Right-click an emote in chat',
  items: [
    row('card', 'Open emote card', 'smile', (c) => ({ label: 'Open emote card', icon: 'smile', run: c.act.card })),
    row('copy', 'Copy name', 'copy', (c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName })),
    row('hide', 'Hide emote', 'eyeOff', (c) => ({ label: `Hide ${c.name}`, icon: 'eyeOff', run: c.act.hide }), { renamable: false }),
    slot('edits', '7TV edits', (c) => c.edits),
    row('blur', 'Blur unlisted emotes', 'eye', (c) => (c.unlisted ? { label: 'Blur unlisted emotes', icon: 'eye', checked: c.blurred, run: c.act.blur } : null), {
      when: 'unlisted emotes',
    }),
    // Palette only (3e).
    row('favorite', 'Favorite', 'star', (c) => ({ label: c.favorite ? 'Unfavorite' : 'Favorite', icon: c.favorite ? 'starFilled' : 'star', run: c.act.favorite }), {
      renamable: false,
    }),
    row('copyLink', 'Copy emote link', 'link', (c) => ({ label: 'Copy emote link', icon: 'link', run: c.act.copyLink })),
  ],
  defaults: [item('card'), item('copy'), item('hide'), group('seventv', '7TV', [item('edits')], 'seventv'), item('blur')],
};

export function emoteSample(who: Who): EmoteCtx {
  return {
    name: 'Kappa',
    unlisted: who === 'mod',
    blurred: true,
    favorite: false,
    edits: who === 'out' ? [] : [{ label: 'Add to 7TV channel', icon: 'seventv', run: noop }],
    act: { card: noop, copyName: noop, hide: noop, blur: noop, favorite: noop, copyLink: noop },
  };
}
