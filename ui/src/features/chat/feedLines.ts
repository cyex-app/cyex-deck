// What a line does in a feed tab (mentions, custom feeds): like in its own chat, with the
// chat being the line's channel. Parts act through lineEvents.ts; the menus are the ones
// that need no chat view (no mod buttons or commands: the usercard has those).

import type { MenuEntry } from '../../components/menu.svelte';
import { hideTip } from '../../components/tooltip.svelte';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { chatIpc } from '../../ipc/chat';
import { accounts } from '../../lib/accounts.svelte';
import { settings } from '../../lib/settings.svelte';
import { copy } from '../../lib/clipboard';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { whispers } from '../whispers/whispers.svelte';
import { FEED_MESSAGE_DEF, FEED_NAME_DEF } from '../../components/menus/defs/feed';
import { menuEntries } from '../../components/menus/use';
import { jumpInLogs } from './inLogs';
import { emoteMenu, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
import { fullMessage } from './fullMessage';
import { plainText } from './text';

/** Each feed line sits in an element with this attribute: the chat it was said in. */
export const FEED_CHAN = 'data-feed-channel';

export interface FeedHost {
  find: (channel: string, id: string) => ChatMessage | undefined;
  openCard: LineHost['openCard'];
  openMenu: (at: { clientX: number; clientY: number }, items: MenuEntry[]) => void;
  openRoom: (channel: string) => void;
  /** V111-53 Shows the line in its chat here, or null when no chat here still has it. */
  inChat: (channel: string, m: ChatMessage) => Promise<(() => void) | null>;
}

const channelOf = (el: Element) => el.closest<HTMLElement>(`[${FEED_CHAN}]`)?.getAttribute(FEED_CHAN) ?? null;

function lineHost(el: HTMLElement, h: FeedHost): LineHost | null {
  const channel = channelOf(el);
  if (!channel) return null;
  return {
    channel,
    account: accounts.current?.userId ?? null,
    senderOf: (e) => {
      const id = e.closest<HTMLElement>('[data-id]')?.dataset.id;
      return id ? h.find(channel, id)?.sender?.id : undefined;
    },
    openCard: h.openCard,
    openMenu: h.openMenu,
  };
}

export function nameEntries(channel: string, login: string, openRoom: (c: string) => void): MenuEntry[] {
  return menuEntries(
    FEED_NAME_DEF,
    {
      whispers: accounts.can('whispers'),
      act: {
        usercard: () => chatIpc.openUsercard(channel, login).catch((e) => toast.error(errorText(e))),
        whisper: () => whispers.show(login),
        openChat: () => openRoom(channel),
        copyName: () => void copy(login, 'Name'),
      },
    },
    { values: { nick: login, 'display-nick': login, chan: channel } },
  );
}

export function messageEntries(channel: string, m: ChatMessage, openRoom: (c: string) => void, inChat: (() => void) | null = null): MenuEntry[] {
  const login = m.sender?.login ?? '';
  const logs = m.sender && !m.notice ? jumpInLogs(channel, m)[0] : undefined;
  return menuEntries(
    FEED_MESSAGE_DEF,
    {
      // V108-N17 Global mentions are rarely in the chat's own scrollback.
      logsLink: !!logs,
      inChat: !!inChat,
      act: {
        openChat: () => openRoom(channel),
        showInChat: () => inChat?.(),
        showInLogs: () => logs?.run(),
        copyMessage: () => void copy(plainText(m.body), 'Message'),
        copyFull: () =>
          void copy(fullMessage(m, settings.v.chat.names, settings.v.chat.timestamps, m.sender ? settings.v.chat.users[m.sender.id] : undefined), 'Message'),
      },
    },
    {
      values: {
        nick: login,
        'display-nick': m.sender?.display ?? login,
        'user-id': m.sender?.id ?? '',
        'msg-id': m.id,
        msg: plainText(m.body),
        chan: channel,
      },
    },
  );
}

export function feedEvents(h: FeedHost) {
  return {
    onclick(e: MouseEvent) {
      const el = (e.target as HTMLElement).closest<HTMLElement>(PARTS);
      const host = el && lineHost(el, h);
      if (el && host) openPart(e, el, host);
    },
    oncontextmenu(e: MouseEvent) {
      const target = e.target as HTMLElement;
      const host = lineHost(target, h);
      if (!host) return;
      const emote = target.closest<HTMLElement>('[data-emote]');
      if (emote) return void emoteMenu(e, emote, host);
      notEmote();
      if (target.closest('a, input, textarea, button')) return;
      const id = target.closest<HTMLElement>('[data-id]')?.dataset.id;
      const m = id ? h.find(host.channel, id) : undefined;
      const name = target.closest<HTMLElement>('[data-user]');
      if (name) {
        e.preventDefault();
        hideTip();
        h.openMenu(e, nameEntries(host.channel, name.dataset.user!, h.openRoom));
      } else if (m && window.getSelection()?.isCollapsed !== false) {
        e.preventDefault();
        hideTip();
        // V111-53 Whether the chat still has it decides the menu: one cached check.
        const at = { clientX: e.clientX, clientY: e.clientY };
        void h.inChat(host.channel, m).then((show) => h.openMenu(at, messageEntries(host.channel, m, h.openRoom, show)));
      }
    },
  };
}
