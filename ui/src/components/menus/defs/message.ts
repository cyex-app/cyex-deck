// V110-U66 Right-click a chat message. The built-in layout, in a fixed order: reply and copy,
// Look up, your own entries and plugins, then the Moderator group (V110-B8: only for an account
// that moderates the chat). Time out and Delete stay on top, after the copy rows.

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
  /** V110-F93 The line was deleted: nothing that needs its text or answers it is offered. */
  deleted?: boolean;
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
    row('reply', 'Reply', 'reply', (c) => (c.line && c.reply && !c.deleted ? { label: 'Reply', icon: 'reply', hint: 'Shift+R-click', run: c.act.reply } : null)),
    row('translate', 'Translate', 'languages', (c) => (c.line && !c.deleted ? { label: c.translated ? 'Translate again' : 'Translate', icon: 'languages', run: c.act.translate } : null), {
      renamable: false,
    }),
    row('copy', 'Copy message', 'copy', (c) => ({ label: 'Copy message', icon: 'copy', run: c.act.copyMessage })),
    row('copyFull', 'Copy with name and time', 'copy', (c) => ({ label: 'Copy with name and time', icon: 'copy', run: c.act.copyFull })),
    row('search', 'Search their messages', 'search', (c) => (c.line && c.search ? { label: 'Search their messages', icon: 'search', run: c.act.search } : null)),
    row('logs', 'Open their logs', 'scroll', line((c) => ({ label: 'Open their logs', icon: 'scroll', run: c.act.logs }))),
    row('showInLogs', 'Show message in logs', 'scroll', (c) => (c.line && c.logsLink ? { label: 'Show message in logs', icon: 'scroll', run: c.act.showInLogs } : null)),
    row('timeout', 'Time out user', 'clock', mod(timeoutEntry), { when: 'mods', mod: true }),
    row('delete', 'Delete message', 'trash', (c) => (c.mod || c.del ? deleteEntry(c.msgId, c.run) : null), { when: 'mods', mod: true }),
    row('pin', 'Pin message', 'pin', (c) => (c.mod ? pinEntry(c.msgId, c.run) : null), { when: 'mods', mod: true }),
    row('pinLine', 'Pin message (broadcaster’s line)', 'pin', (c) => (!c.mod && c.pinLine ? pinEntry(c.msgId, c.run) : null), {
      when: 'on the broadcaster’s line',
      mod: true,
    }),
    row('warn', 'Warn user', 'alert', mod(warnEntry), { when: 'mods', mod: true }),
    row('unban', 'Unban user', 'undo', mod(unbanEntry), { when: 'mods', mod: true }),
    row('monitor', 'Monitor user', 'eye', mod(monitorEntry), { when: 'mods', mod: true }),
    row('restrict', 'Restrict user', 'eyeOff', mod(restrictEntry), { when: 'mods', mod: true }),
    row('mod', 'Mod or Unmod', 'shield', mod(modRoleEntry), { when: 'mods', mod: true, renamable: false }),
    row('vip', 'VIP or Unvip', 'gem', mod(vipRoleEntry), { when: 'mods', mod: true, renamable: false }),
    row('ban', 'Ban user', 'gavel', mod(banEntry), { when: 'mods', mod: true }),
    // Palette only (3e).
    row('copyId', 'Copy message ID', 'copy', (c) => (c.msgId ? { label: 'Copy message ID', icon: 'copy', run: c.act.copyId } : null)),
    row('copyLogsLink', 'Copy logs link', 'link', (c) => (c.logsLink ? { label: 'Copy logs link', icon: 'link', run: c.act.copyLogsLink } : null)),
    row('mention', 'Mention', 'reply', (c) => (c.line && c.mention ? { label: 'Mention', icon: 'reply', run: c.act.mention } : null)),
    row('copyName', 'Copy name', 'copy', line((c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName }))),
    row('hideUser', 'Hide user for me', 'eyeOff', line((c) => ({ label: 'Hide user for me', icon: 'eyeOff', note: 'Only for you, not on Twitch', run: c.act.hideUser }))),
    slot('entries', 'Your entries', (c) => c.entries),
    slot('plugins', 'Plugin entries', (c) => c.plugins),
  ],
  defaults: [
    item('reply'),
    item('translate'),
    item('copy'),
    item('copyFull'),
    // Time out and Delete stay on top for a moderator (both are mod-only rows).
    item('timeout'),
    item('delete'),
    SEP,
    group('lookup', 'Look up', [item('search'), item('logs'), item('showInLogs')], 'search'),
    SEP,
    item('entries'),
    item('plugins'),
    SEP,
    group(
      'moderate',
      'Moderator',
      [item('pin'), item('pinLine'), item('warn'), item('unban'), item('monitor'), item('restrict'), SEP, item('mod'), item('vip'), SEP, item('ban')],
      'shield',
    ),
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
