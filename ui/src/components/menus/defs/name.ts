// V110-U66 Right-click a name, in the chat and in the viewer list. The built-in layout keeps
// the essentials on top and puts the rest in groups.

import {
  banEntry,
  modRoleEntry,
  monitorEntry,
  restrictEntry,
  timeoutEntry,
  unbanEntry,
  vipRoleEntry,
  warnEntry,
  type ModUser,
} from '../../../features/modview/modEntries.ts';
import type { MenuEntry } from '../../menuItems.ts';
import { group, item, type MenuDef, SEP } from '../types.ts';
import { noop, row, slot, type Who } from './make.ts';

export interface NameCtx {
  login: string;
  /** An account is in this chat. */
  signedIn: boolean;
  /** The account can whisper. */
  whispers: boolean;
  /** The chat takes a mention (not every viewer list does). */
  mention: boolean;
  /** The chat can search its lines. */
  search: boolean;
  /** Their user id is known. */
  userId: boolean;
  /** Mod actions on them; `null` when the account may not. */
  mod: ModUser | null;
  act: {
    usercard(): void;
    mention(): void;
    whisper(): void;
    search(): void;
    logs(): void;
    copyName(): void;
    hide(): void;
    block(): void;
    unblock(): void;
    openChat(): void;
    watch(): void;
    hideHere(): void;
    copyId(): void;
  };
  /** S1 Your entries. */
  entries: MenuEntry[];
  /** S5 Plugins' entries. */
  plugins: MenuEntry[];
}

const mod = (f: (m: ModUser) => MenuEntry | null) => (c: NameCtx) => (c.mod ? f(c.mod) : null);

export const NAME_DEF: MenuDef<NameCtx> = {
  id: 'chat.name',
  label: 'Names',
  where: 'Right-click a name in chat or in the viewer list',
  items: [
    row('usercard', 'Usercard', 'user', (c) => ({ label: 'Usercard', icon: 'user', run: c.act.usercard })),
    row('mention', 'Mention', 'reply', (c) => (c.mention ? { label: 'Mention', icon: 'reply', run: c.act.mention } : null)),
    row('whisper', 'Whisper', 'whisper', (c) => (c.whispers ? { label: 'Whisper', icon: 'whisper', run: c.act.whisper } : null), {
      when: 'signed in',
    }),
    row('search', 'Their messages', 'search', (c) => (c.search ? { label: 'Their messages', icon: 'search', run: c.act.search } : null)),
    row('logs', 'Open their logs', 'scroll', (c) => ({ label: 'Open their logs', icon: 'scroll', run: c.act.logs })),
    row('copy', 'Copy name', 'copy', (c) => ({ label: 'Copy name', icon: 'copy', run: c.act.copyName })),
    row('timeout', 'Time out', 'clock', mod(timeoutEntry), { when: 'mods' }),
    row('ban', 'Ban', 'gavel', mod(banEntry), { when: 'mods' }),
    row('warn', 'Warn', 'alert', mod(warnEntry), { when: 'mods' }),
    row('unban', 'Unban', 'undo', mod(unbanEntry), { when: 'mods' }),
    row('monitor', 'Monitor', 'eye', mod(monitorEntry), { when: 'mods' }),
    row('restrict', 'Restrict', 'eyeOff', mod(restrictEntry), { when: 'mods' }),
    row('mod', 'Mod or Unmod', 'shield', mod(modRoleEntry), { when: 'mods', renamable: false }),
    row('vip', 'VIP or Unvip', 'gem', mod(vipRoleEntry), { when: 'mods', renamable: false }),
    row('hide', 'Hide', 'eyeOff', (c) => ({ label: 'Hide', icon: 'eyeOff', note: 'Only for you, not on Twitch', run: c.act.hide })),
    row(
      'block',
      'Block',
      'ban',
      (c) => (c.signedIn ? { label: 'Block', icon: 'ban', note: 'Blocks them on Twitch', danger: true, run: c.act.block } : null),
      { when: 'signed in' },
    ),
    // Palette only (3e).
    row('openChat', 'Open their chat', 'popout', (c) => ({ label: 'Open their chat', icon: 'popout', run: c.act.openChat })),
    row('watch', 'Watch their stream', 'tv', (c) => (c.userId ? { label: 'Watch their stream', icon: 'tv', run: c.act.watch } : null)),
    row('hideHere', 'Hide in this chat', 'eyeOff', (c) => ({
      label: 'Hide in this chat',
      icon: 'eyeOff',
      note: 'Only for you, only here',
      run: c.act.hideHere,
    })),
    row('copyId', 'Copy user ID', 'copy', (c) => (c.userId ? { label: 'Copy user ID', icon: 'copy', run: c.act.copyId } : null)),
    row('shoutout', 'Shoutout', 'zap', mod((m) => ({ label: 'Shoutout', icon: 'zap', run: () => m.run(`/shoutout ${m.login}`) })), {
      when: 'mods',
    }),
    row('unblock', 'Unblock', 'undo', (c) => (c.signedIn ? { label: 'Unblock', icon: 'undo', run: c.act.unblock } : null), {
      when: 'signed in',
    }),
    slot('entries', 'Your entries', (c) => c.entries),
    slot('plugins', 'Plugin entries', (c) => c.plugins),
  ],
  defaults: [
    item('usercard'),
    item('mention'),
    item('whisper'),
    item('copy'),
    SEP,
    item('timeout'),
    item('ban'),
    SEP,
    group('moderate', 'Moderate', [item('warn'), item('unban'), item('monitor'), item('restrict'), SEP, item('mod'), item('vip')], 'shield'),
    SEP,
    group('lookup', 'Look up', [item('search'), item('logs')], 'search'),
    SEP,
    group('hideblock', 'Hide or block', [item('hide'), item('block')], 'eyeOff'),
    SEP,
    item('entries'),
    item('plugins'),
  ],
};

/** A fake mod target for previews and tests. */
export function sampleMod(login = 'viewer'): ModUser {
  return {
    login,
    roles: true,
    onMod: { offered: true },
    presets: [
      { label: '10s', secs: 10 },
      { label: '10m', secs: 600 },
      { label: '1h', secs: 3600 },
    ],
    run: noop,
    warn: noop,
  };
}

export function nameSample(who: Who): NameCtx {
  const a = who !== 'out';
  return {
    login: 'viewer',
    signedIn: a,
    whispers: a,
    mention: true,
    search: true,
    userId: true,
    mod: who === 'mod' ? sampleMod() : null,
    act: {
      usercard: noop,
      mention: noop,
      whisper: noop,
      search: noop,
      logs: noop,
      copyName: noop,
      hide: noop,
      block: noop,
      unblock: noop,
      openChat: noop,
      watch: noop,
      hideHere: noop,
      copyId: noop,
    },
    entries: [],
    plugins: [],
  };
}
