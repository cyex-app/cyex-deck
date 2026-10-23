// What a line does in a feed tab (mentions, custom feeds): like in its own chat, with the
// chat being the line's channel. Parts act through lineEvents.ts; the menus are the ones
// that need no chat view (no mod buttons or commands: the usercard has those).

import type { MenuEntry } from '../../components/menu.svelte';
import { hideTip } from '../../components/tooltip.svelte';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import { chatIpc } from '../../ipc/chat';
import { accounts } from '../../lib/accounts.svelte';
import { copy } from '../../lib/clipboard';
import { errorText } from '../../lib/errors';
import { toast } from '../../lib/toast.svelte';
import { whispers } from '../whispers/whispers.svelte';
import { emoteMenu, notEmote, openPart, PARTS, type LineHost } from './lineEvents';
import { plainText } from './text';

/** Each feed line sits in an element with this attribute: the chat it was said in. */
export const FEED_CHAN = 'data-feed-channel';

export interface FeedHost {
  find: (channel: string, id: string) => ChatMessage | undefined;
  openCard: LineHost['openCard'];
  openMenu: LineHost['openMenu'];
  openRoom: (channel: string) => void;
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
  return [
    {
      label: 'Usercard',
      icon: 'user',
      run: () => chatIpc.openUsercard(channel, login).catch((e) => toast.error(errorText(e))),
    },
    ...(accounts.can('whispers') ? [{ label: 'Whisper', icon: 'whisper' as const, run: () => whispers.show(login) }] : []),
    { label: 'Copy name', icon: 'copy', run: () => void copy(login, 'Name') },
    'separator',
    { label: 'Open this chat', icon: 'list', run: () => openRoom(channel) },
  ];
}

export function messageEntries(channel: string, m: ChatMessage, openRoom: (c: string) => void): MenuEntry[] {
  return [
    { label: 'Copy message', icon: 'copy', run: () => void copy(plainText(m.body), 'Message') },
    { label: 'Open this chat', icon: 'list', run: () => openRoom(channel) },
  ];
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
        h.openMenu(e, messageEntries(host.channel, m, h.openRoom));
      }
    },
  };
}
