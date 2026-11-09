// V110-U66 Right-click a clip, in Clips and on a channel's clips page. The page has no rename,
// edit or delete, so those rows only show in Clips.

import { item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface ClipCtx {
  /** A save to MP4 is running; its text, e.g. "42 %". */
  saving: string | null;
  /** The clip can be renamed here. */
  rename: boolean;
  /** Made by this account: opens Twitch's editor. */
  editOnTwitch: boolean;
  /** The list can delete clips. */
  remove: boolean;
  act: { play(): void; save(): void; stop(): void; copyLink(): void; rename(): void; edit(): void; remove(): void };
}

export const CLIP_DEF: MenuDef<ClipCtx> = {
  id: 'clips.clip',
  label: 'Clips',
  where: 'Right-click a clip',
  items: [
    row('play', 'Play', 'play', (c) => ({ label: 'Play', icon: 'play', run: c.act.play })),
    row('save', 'Save as MP4', 'download', (c) =>
      c.saving === null ? { label: 'Save as MP4', icon: 'download', run: c.act.save } : { label: 'Stop saving', icon: 'x', hint: c.saving, run: c.act.stop },
    { renamable: false }),
    row('copy', 'Copy link', 'link', (c) => ({ label: 'Copy link', icon: 'link', run: c.act.copyLink })),
    row('rename', 'Rename', 'pencil', (c) => (c.rename ? { label: 'Rename', icon: 'pencil', run: c.act.rename } : null), { when: 'your clips' }),
    row('edit', 'Edit on Twitch', 'scissors', (c) =>
      c.editOnTwitch ? { label: 'Edit on Twitch', icon: 'scissors', note: 'Title, start and end, in Twitch’s editor', run: c.act.edit } : null,
    { when: 'your clips' }),
    row('delete', 'Delete clip', 'trash', (c) =>
      c.remove ? { label: 'Delete clip', icon: 'trash', danger: true, note: 'Gone for good.', confirm: 'Click to delete', run: c.act.remove } : null,
    { when: 'in Clips' }),
  ],
  defaults: [item('play'), item('save'), item('copy'), item('rename'), item('edit'), SEP, item('delete')],
};

export function clipSample(who: Who): ClipCtx {
  return {
    saving: null,
    rename: who !== 'out',
    editOnTwitch: who !== 'out',
    remove: true,
    act: { play: noop, save: noop, stop: noop, copyLink: noop, rename: noop, edit: noop, remove: noop },
  };
}
