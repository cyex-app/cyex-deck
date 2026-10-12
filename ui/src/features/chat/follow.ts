// V103-22 The mod view's and the stream manager's own chat follow the channel's regular
// chat (tabs, popped-out chats, `#channel` chats). Rust says where those show and which
// windows are still loading (`chat:shown`); these read it for one window.

import type { ChatShown } from '../../ipc/bindings/ChatShown';

/** Nothing but this window is loading, so every chat that will show already does. */
export function settled(s: ChatShown, own: string): boolean {
  return s.loading.every((w) => w === own);
}

/** `login`'s regular chat shows in a window other than `own`. */
export function elsewhere(chats: ChatShown['chats'], login: string, own: string): boolean {
  const key = login.trim().replace(/^#/, '').toLowerCase();
  return (chats[key] ?? []).some((w) => w !== own);
}
