// Which name a chatter shows under: local nickname (C3), display name or login, either first (C20).

import type { NameStyle } from '../../ipc/bindings/NameStyle';
import type { UserNote } from '../../ipc/bindings/UserNote';

export interface Who {
  login: string;
  display: string;
}

export interface ShownName {
  name: string;
  /** Shown dimmed in brackets after the name; only when it differs beyond letter case. */
  extra?: string;
}

export function isLocalized(w: Who): boolean {
  return w.display.toLowerCase() !== w.login.toLowerCase();
}

export function shownName(w: Who, style: NameStyle, note?: UserNote): ShownName {
  if (note?.nickname) return { name: note.nickname };
  const localized = isLocalized(w);
  switch (style) {
    case 'display':
      return { name: w.display };
    case 'displayLogin':
      return localized ? { name: w.display, extra: w.login } : { name: w.display };
    case 'username':
      return { name: w.login };
    case 'usernameDisplay':
      return localized ? { name: w.login, extra: w.display } : { name: w.login };
  }
}

/** V110-F05 The name a reply header gives its parent: by the Names setting, like a chat line. */
export function replyParentName(r: { parentLogin: string; parentDisplay: string }, style: NameStyle): string {
  return shownName({ login: r.parentLogin || r.parentDisplay, display: r.parentDisplay || r.parentLogin }, style).name;
}
