// Right-click a name, in the chat and in the viewer list: the same entries in the same
// order (V101-30): main action, others, go somewhere, copy, then red last.

import type { MenuEntry } from '../../components/menu.svelte';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { UiAction } from '../../ipc/bindings/UiAction';
import { chatIpc } from '../../ipc/chat';
import { pluginsIpc } from '../../ipc/plugins';
import { accounts } from '../../lib/accounts.svelte';
import { copy } from '../../lib/clipboard';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { may, whose } from '../modview/may';
import { mods, timeouts } from '../modview/moderation.svelte';
import { onModTarget } from '../modview/lineMod';
import type { Standing } from '../modview/standing';
import { standings } from '../modview/standings.svelte';
import { pluginItems } from '../plugins/pluginMenus';
import { openChannel } from '../browse/open.svelte';
import { tabs } from '../tabs/tabs.svelte';
import { popOut } from '../../app/panels';
import { NAME_DEF, type NameCtx } from '../../components/menus/defs/name';
import { menuEntries } from '../../components/menus/use';
import { roomRoute } from './roomRoute';
import { plugins } from '../plugins/plugins.svelte';
import { whispers } from '../whispers/whispers.svelte';
import { runEntry, toItems, userMenus } from './menus.svelte';
import { openInLogs } from './inLogs';
import { plainText } from './text';
import { blockUser, hideUser, type Run, withUndo } from './undo';

export interface NameMenu {
  /** The chat's channel login. */
  channel: string;
  account: string | null;
  /** The chat view, when there is one (not in the mod view); your own entries need it. */
  view: number | null;
  login: string;
  /** The line the name was on, if any. */
  msg?: ChatMessage;
  standing?: Standing;
  run: Run;
  onaction?: (a: UiAction) => void;
  onmention?: (login: string) => void;
  /** The reason for a warning goes in the message box; without it the usercard asks. */
  onprefill?: (text: string) => void;
}

export function nameMenu(c: NameMenu): MenuEntry[] {
  const { login, channel, msg } = c;
  const mine = msg?.sender?.login === login ? msg.sender : null;
  const ctx = {
    login,
    userId: mine?.id ?? null,
    display: mine?.display ?? null,
    msgId: msg?.id ?? null,
    msg: msg ? plainText(msg.body) : null,
    msgTime: msg?.ts ?? null,
  };
  const own = userMenus.current.user;
  // V107-R14 Rust's answer: everyone but the channel itself.
  const caps = mods.caps(c.account, channel);
  const canMod = caps && may(caps, 'user', whose({ id: '', login }, c.account, channel)).offered;
  const open = () => chatIpc.openUsercard(channel, login).catch((e) => toast.error(errorText(e)));
  const warn = c.onprefill ? () => c.onprefill!(`/warn ${login} `) : open;
  const standing = standings.of(c.channel, login, c.standing);
  const userId = ctx.userId;
  const run = (line: string) => void c.run(line);
  const menu: NameCtx = {
    login,
    signedIn: !!c.account,
    whispers: accounts.can('whispers'),
    mention: !!c.onmention,
    search: !!c.onaction,
    userId: !!userId,
    mod:
      canMod && caps
        ? { login, roles: caps.roles, onMod: onModTarget(caps, standing?.mod), standing, presets: timeouts(), run: c.run, warn }
        : null,
    act: {
      usercard: open,
      mention: () => c.onmention?.(login),
      whisper: () => whispers.show(login),
      search: () => c.onaction?.({ type: 'search', query: `from:${login}` }),
      logs: () => openInLogs(channel, login),
      copyName: () => void copy(login, 'Name'),
      hide: () => void hideUser(c.run, login),
      block: () => void blockUser(c.run, login),
      unblock: () => run(`/unblock ${login}`),
      openChat: () => void popOut(roomRoute(login, c.account), `${login} chat`),
      watch: () =>
        void openChannel({ show: (id) => tabs.activate(id), account: () => c.account }, { id: userId!, login, name: ctx.display ?? login }),
      hideHere: () => void withUndo(`${login} is hidden in this chat.`, () => c.run(`/hideuser ${login} here`), () => c.run(`/unhideuser ${login}`)),
      copyId: () => void copy(userId!, 'User ID'),
    },
    entries: own.length && c.view !== null ? toItems(own, (cmd) => runEntry(c.view!, cmd, ctx, c.onaction)) : [],
    plugins: pluginItems(plugins.menus, 'name', null, (p) =>
      pluginsIpc.runName(p.plugin, p.index, channel, login, c.view, msg?.id ?? null).catch((err) => toast.error(errorText(err))),
    ),
  };
  return menuEntries(NAME_DEF, menu, {
    view: c.view,
    context: ctx,
    onaction: c.onaction,
    prefill: c.onprefill,
    values: {
      nick: login,
      'display-nick': ctx.display ?? login,
      'user-id': userId ?? '',
      'msg-id': ctx.msgId ?? '',
      msg: ctx.msg ?? '',
      chan: channel,
    },
  });
}
