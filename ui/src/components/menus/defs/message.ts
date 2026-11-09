// V110-U66 Right-click a chat message. The built-in layout: reply, copy, translate on top, the
// mod rows in a group of their own, the less used ones under More.

import {
  banEntry,
  deleteEntry,
  modRoleEntry,
  monitorEntry,
  pinEntry,
  restrictEntry,
  timeoutEntry,
  unbanEntry,
  vipRoleEntry,
  warnEntry,
  type ModUser,
  type Run,
} from '../../../features/modview/modEntries.ts';
import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type MenuDef, SEP } from '../types.ts';
import { noop, row, slot, type Who } from './make.ts';
import { sampleMod } from './name.ts';

export interface MessageCtx {
  login: string;
  msgId: string;
  /** A user's line (not a notice or a system line). */
  line: boolean;
  /** The chat takes replies and the account may reply. */
  reply: boolean;
  /** Already translated: the row says "Translate again". */
  translated: boolean;
  /** The chat can search its lines. */
  search: boolean;
  /** The chat takes a mention. */
  mention: boolean;
  /** The line has a place on logs.cyex.app. */
  logsLink: boolean;
  /** Mod actions on the sender; `null` when the account may not. */
  mod: ModUser | null;
  /** The sender is the broadcaster and the account may pin their line. */
  pinLine: boolean;
  /** The account may delete the line. */
  del: boolean;
  /** Runs a chat line (`/pin …`, `/delete …`). */
  run: Run;
  act: {
    reply(): void;
    translate(): void;
    search(): void;
    logs(): void;
    showInLogs(): void;
    copyMessage(): void;
    copyFull(): void;
    copyId(): void;
    copyLogsLink(): void;
    mention(): void;
    copyName(): void;
    hideUser(): void;
  };
  entries: MenuEntry[];
  plugins: MenuEntry[];
}

const mod = (f: (m: ModUser) => MenuEntry | null) => (c: MessageCtx) => (c.mod ? f(c.mod) : null);
const line = (f: (c: MessageCtx) => MenuEntry) => (c: MessageCtx) => (c.line ? f(c) : null);

export const MESSAGE_DEF: MenuDef<MessageCtx> = {
  id: 'chat.message',
  label: 'Messages',
  where: 'Right-click a chat message',
  items: [
    row('reply', 'Reply', 'reply', (c) => (c.line && c.reply ? { label: 'Reply', icon: 'reply', hint: 'Shift+R-click', run: c.act.reply } : null)),
    row('translate', 'Translate', 'languages', line((c) => ({ label: c.translated ? 'Translate again' : 'Translate', icon: 'languages', run: c.act.translate })), {
      renamable: false,
    }),
    row('copy', 'Copy message', 'copy', (c) => ({ label: 'Copy message', icon: 'copy', run: c.act.copyMessage })),
    row('copyFull', 'Copy full message', 'copy', (c) => ({ label: 'Copy full message', icon: 'copy', run: c.act.copyFull })),
    row('search', 'Their messages', 'search', (c) => (c.line && c.search ? { label: 'Their messages', icon: 'search', run: c.act.search } : null)),
    row('logs', 'Open their logs', 'scroll', line((c) => ({ label: 'Open their logs', icon: 'scroll', run: c.act.logs }))),
    row('showInLogs', 'Show in logs', 'scroll', (c) => (c.line && c.logsLink ? { label: 'Show in logs', icon: 'scroll', run: c.act.showInLogs } : null)),
    row('timeout', 'Time out', 'clock', mod(timeoutEntry), { when: 'mods' }),
    row('delete', 'Delete message', 'trash', (c) => (c.mod || c.del ? deleteEntry(c.msgId, c.run) : null), { when: 'mods' }),
    row('pin', 'Pin message', 'pin', (c) => (c.mod ? pinEntry(c.msgId, c.run) : null), { when: 'mods' }),
    row('pinLine', 'Pin message (their line)', 'pin', (c) => (!c.mod && c.pinLine ? pinEntry(c.msgId, c.run) : null), {
      when: 'on the broadcaster’s line',
    }),
    row('warn', 'Warn', 'alert', mod(warnEntry), { when: 'mods' }),
    row('unban', 'Unban', 'undo', mod(unbanEntry), { when: 'mods' }),
    row('monitor', 'Monitor', 'eye', mod(monitorEntry), { when: 'mods' }),
    row('restrict', 'Restrict', 'eyeOff', mod(restrictEntry), { when: 'mods' }),
    row('mod', 'Mod or Unmod', 'shield', mod(modRoleEntry), { when: 'mods', renamable: false }),
    row('vip', 'VIP or Unvip', 'gem', mod(vipRoleEntry), { when: 'mods', renamable: false }),
    row('ban', 'Ban', 'gavel', mod(banEntry), { when: 'mods' }),
    // Palette only (3e).
    row('copyId', 'Copy message ID', 'copy', (c) => (c.msgId ? { label: 'Copy message ID', icon: 'copy', run: c.act.copyId } : null)),
    row('copyLogsLink', 'Copy logs link', 'link', (c) => (c.logsLink ? { label: 'Copy logs link', icon: 'link', run: c.act.copyLogsLink } : null)),
    row('mention', 'Mention', 'reply', (c) => (c.line && c.mention ? { label: 'Mention', icon: 'reply', run: c.act.mention } : null)),
    row('copyName', 'Copy name', 'copy', line((c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName }))),
    row('hideUser', 'Hide this user', 'eyeOff', line((c) => ({ label: 'Hide this user', icon: 'eyeOff', note: 'Only for you, not on Twitch', run: c.act.hideUser }))),
    slot('entries', 'Your entries', (c) => c.entries),
    slot('plugins', 'Plugin entries', (c) => c.plugins),
  ],
  defaults: [
    item('reply'),
    item('copy'),
    item('translate'),
    SEP,
    item('timeout'),
    item('pinLine'),
    item('delete'),
    SEP,
    group('moderate', 'Moderate', [item('pin'), item('warn'), item('unban'), item('monitor'), item('restrict'), item('mod'), item('vip'), SEP, item('ban')], 'shield'),
    SEP,
    group('more', 'More', [item('copyFull'), item('search'), item('logs'), item('showInLogs')], 'more'),
    SEP,
    item('entries'),
    item('plugins'),
  ],
};

export function messageSample(who: Who): MessageCtx {
  const a = who !== 'out';
  return {
    login: 'viewer',
    msgId: 'm1',
    line: true,
    reply: a,
    translated: false,
    search: true,
    mention: true,
    logsLink: true,
    mod: who === 'mod' ? sampleMod() : null,
    pinLine: false,
    del: who === 'mod',
    run: noop,
    act: {
      reply: noop,
      translate: noop,
      search: noop,
      logs: noop,
      showInLogs: noop,
      copyMessage: noop,
      copyFull: noop,
      copyId: noop,
      copyLogsLink: noop,
      mention: noop,
      copyName: noop,
      hideUser: noop,
    },
    entries: [],
    plugins: [],
  };
}
