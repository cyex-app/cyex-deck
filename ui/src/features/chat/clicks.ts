// Clicks on names and links in lines shown outside a chat's own list (search, feeds).
// Each line sits in an element with `data-channel`, the chat it was said in.

import { chatIpc } from '../../ipc/chat';
import { systemIpc } from '../../ipc/system';
import { toast } from '../../lib/toast.svelte';

export function lineClick(e: MouseEvent): void {
  const el = (e.target as HTMLElement).closest<HTMLElement>('[data-user],[data-link]');
  const channel = el?.closest<HTMLElement>('[data-channel]')?.dataset.channel;
  if (!el || !channel) return;
  const d = el.dataset;
  if (d.link) {
    e.preventDefault();
    systemIpc.openChatLink(d.link).catch((err) => toast.error(String(err)));
  } else if (d.user) {
    chatIpc.openUsercard(channel, d.user).catch((err) => toast.error(String(err)));
  }
}
