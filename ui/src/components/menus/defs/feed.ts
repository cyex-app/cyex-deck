// V110-U66 Right-click a name or a message in a feed tab (mentions, custom feeds). The layouts
// are the built-in ones, unchanged; they can still be rearranged.

import { item, type MenuDef, SEP } from '../types.ts';
import { noop, row, type Who } from './make.ts';

export interface FeedNameCtx {
  /** The account can whisper. */
  whispers: boolean;
  act: { usercard(): void; whisper(): void; openChat(): void; copyName(): void };
}

export const FEED_NAME_DEF: MenuDef<FeedNameCtx> = {
  id: 'feed.name',
  label: 'Names in feeds',
  where: 'Right-click a name in a feed tab',
  items: [
    row('usercard', 'Open usercard', 'user', (c) => ({ label: 'Open usercard', icon: 'user', run: c.act.usercard })),
    row('whisper', 'Whisper', 'whisper', (c) => (c.whispers ? { label: 'Whisper', icon: 'whisper', run: c.act.whisper } : null), { when: 'signed in' }),
    row('openChat', 'Open this chat', 'list', (c) => ({ label: 'Open this chat', icon: 'list', run: c.act.openChat })),
    row('copy', 'Copy name', 'copy', (c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName })),
  ],
  defaults: [item('usercard'), item('whisper'), SEP, item('openChat'), item('copy')],
};

export function feedNameSample(who: Who): FeedNameCtx {
  return { whispers: who !== 'out', act: { usercard: noop, whisper: noop, openChat: noop, copyName: noop } };
}

export interface FeedMessageCtx {
  /** The line has a place on logs.cyex.app. */
  logsLink: boolean;
  act: { openChat(): void; showInLogs(): void; copyMessage(): void; copyFull(): void };
}

export const FEED_MESSAGE_DEF: MenuDef<FeedMessageCtx> = {
  id: 'feed.message',
  label: 'Messages in feeds',
  where: 'Right-click a message in a feed tab',
  items: [
    row('openChat', 'Open this chat', 'list', (c) => ({ label: 'Open this chat', icon: 'list', run: c.act.openChat })),
    row('showInLogs', 'Show message in logs', 'scroll', (c) => (c.logsLink ? { label: 'Show message in logs', icon: 'scroll', run: c.act.showInLogs } : null)),
    row('copy', 'Copy message', 'copy', (c) => ({ label: 'Copy message', icon: 'copy', run: c.act.copyMessage })),
    row('copyFull', 'Copy with name and time', 'copy', (c) => ({ label: 'Copy with name and time', icon: 'copy', run: c.act.copyFull })),
  ],
  defaults: [item('openChat'), item('showInLogs'), item('copy'), item('copyFull')],
};

export function feedMessageSample(_who: Who): FeedMessageCtx {
  return { logsLink: true, act: { openChat: noop, showInLogs: noop, copyMessage: noop, copyFull: noop } };
}
