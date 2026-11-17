// V110-U66 The ⋯ menu of a whisper thread. Built-in layout unchanged.

import { item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface WhisperCtx {
  /** The thread is in the history (saved), so it can be archived, muted or taken back. */
  history: boolean;
  archived: boolean;
  /** You whispered them first, so they may whisper back for 48 hours. */
  temporary: boolean;
  muted: boolean;
  /** V111-47 The account whose sign-in the saved-whispers items need (they lock without Extra Twitch features). */
  account?: string | null;
  act: { archive(): void; revoke(): void; mute(): void; block(): void };
}

export const WHISPER_DEF: MenuDef<WhisperCtx> = {
  id: 'whispers.thread',
  label: 'Whisper threads',
  where: 'The ⋯ button in a whisper thread',
  items: [
    row('archive', 'Archive', 'archive', (c) =>
      c.history
        ? { label: c.archived ? 'Move back to the list' : 'Archive', icon: 'archive', run: c.act.archive }
        : { label: 'Archive', icon: 'archive', lock: 'gql', lockAccount: c.account, run: c.act.archive }, {
      renamable: false,
    }),
    row('revoke', 'Take back their access', 'lock', (c) =>
      c.history && c.temporary
        ? { label: 'Take back their access', icon: 'lock', note: 'You whispered them, so they can whisper you for 48 hours.', run: c.act.revoke }
        : null,
    ),
    row('mute', 'Mute alerts', 'bellOff', (c) =>
      c.history
        ? { label: c.muted ? 'Unmute alerts' : 'Mute alerts', icon: c.muted ? 'bell' : 'bellOff', run: c.act.mute }
        : { label: 'Mute alerts', icon: 'bellOff', lock: 'gql', lockAccount: c.account, run: c.act.mute }, {
      renamable: false,
    }),
    row('block', 'Block on Twitch', 'ban', (c) => ({ label: 'Block on Twitch', icon: 'ban', note: 'Blocks them on Twitch', danger: true, run: c.act.block })),
  ],
  defaults: [item('archive'), item('revoke'), item('mute'), SEP, item('block')],
};

export function whisperSample(who: Who): WhisperCtx {
  return { history: who !== 'out', archived: false, temporary: who === 'mod', muted: false, act: { archive: noop, revoke: noop, mute: noop, block: noop } };
}
