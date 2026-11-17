// V110-U66 Right-click a name or a message in a feed tab (mentions, custom feeds). V111-10 Names: usercard, whisper, Open chat,
// Copy name. Messages: the copy rows, Open chat, Show message in logs. V111-53 Show in chat, while the line is still in its chat here. They can still be rearranged.

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
    row('openChat', 'Open chat', 'list', (c) => ({ label: 'Open chat', icon: 'list', run: c.act.openChat })),
    row('copy', 'Copy name', 'copy', (c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName })),
  ],
  defaults: [item('usercard'), item('whisper'), item('openChat'), SEP, item('copy')],
};

export function feedNameSample(who: Who): FeedNameCtx {
  return { whispers: who !== 'out', act: { usercard: noop, whisper: noop, openChat: noop, copyName: noop } };
}

export interface FeedMessageCtx {
  /** The line has a place on logs.cyex.app. */
  logsLink: boolean;
  /** V111-53 The line is still in its chat, in a tab here. */
  inChat: boolean;
  act: { openChat(): void; showInChat(): void; showInLogs(): void; copyMessage(): void; copyFull(): void };
}

export const FEED_MESSAGE_DEF: MenuDef<FeedMessageCtx> = {
  id: 'feed.message',
  label: 'Messages in feeds',
  where: 'Right-click a message in a feed tab',
  items: [
    row('openChat', 'Open chat', 'list', (c) => ({ label: 'Open chat', icon: 'list', run: c.act.openChat })),
    row('showInChat', 'Show in chat', 'chat', (c) => (c.inChat ? { label: 'Show in chat', icon: 'chat', run: c.act.showInChat } : null), { when: 'still in its chat' }),
    row('showInLogs', 'Show message in logs', 'scroll', (c) => (c.logsLink ? { label: 'Show message in logs', icon: 'scroll', run: c.act.showInLogs } : null)),
    row('copy', 'Copy message', 'copy', (c) => ({ label: 'Copy message', icon: 'copy', run: c.act.copyMessage })),
    row('copyFull', 'Copy full message', 'copy', (c) => ({ label: 'Copy full message', icon: 'copy', run: c.act.copyFull })),
  ],
  defaults: [item('copy'), item('copyFull'), SEP, item('openChat'), item('showInChat'), item('showInLogs')],
};

export function feedMessageSample(_who: Who): FeedMessageCtx {
  return { logsLink: true, inChat: true, act: { openChat: noop, showInChat: noop, showInLogs: noop, copyMessage: noop, copyFull: noop } };
}
