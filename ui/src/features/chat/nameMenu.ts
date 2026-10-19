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
import { mods, userItems } from '../modview/moderation.svelte';
import type { Standing } from '../modview/standing';
import { standings } from '../modview/standings.svelte';
import { pluginItems } from '../plugins/pluginMenus';
import { plugins } from '../plugins/plugins.svelte';
import { whispers } from '../whispers/whispers.svelte';
import { runEntry, toItems, userMenus } from './menus.svelte';
import { plainText } from './text';
import { blockUser, hideUser, type Run } from './undo';

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
  const role = mods.role(c.account, channel);
  const canMod = role && login !== channel;
  const open = () => chatIpc.openUsercard(channel, login).catch((e) => toast.error(errorText(e)));
  const warn = c.onprefill ? () => c.onprefill!(`/warn ${login} `) : open;
  return [
    { label: 'Usercard', icon: 'user', run: open },
    ...(c.onmention ? [{ label: 'Mention', icon: 'reply' as const, run: () => c.onmention!(login) }] : []),
    ...(accounts.can('whispers') ? [{ label: 'Whisper', icon: 'whisper' as const, run: () => whispers.show(login) }] : []),
    ...(c.onaction
      ? [{ label: 'Their messages', icon: 'search' as const, run: () => c.onaction!({ type: 'search', query: `from:${login}` }) }]
      : []),
    { label: 'Copy name', icon: 'copy', run: () => void copy(login, 'Name') },
    ...(canMod ? ['separator' as const, ...userItems(role, login, c.run, warn, standings.of(c.channel, login, c.standing))] : []),
    'separator',
    { label: 'Hide', icon: 'eyeOff', note: 'Only for you, not on Twitch', run: () => void hideUser(c.run, login) },
    ...(c.account
      ? [{ label: 'Block', icon: 'ban' as const, note: 'Blocks them on Twitch', danger: true, run: () => void blockUser(c.run, login) }]
      : []),
    ...(own.length && c.view !== null
      ? ['separator' as const, ...toItems(own, (cmd) => runEntry(c.view!, cmd, ctx, c.onaction))]
      : []),
    'separator',
    ...pluginItems(plugins.menus, 'name', null, (p) =>
      pluginsIpc.runName(p.plugin, p.index, channel, login, c.view, msg?.id ?? null).catch((err) => toast.error(errorText(err))),
    ),
  ];
}
