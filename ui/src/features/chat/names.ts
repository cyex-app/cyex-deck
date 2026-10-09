// Which name a chatter shows under: local nickname (C3), display name or login (C20).

import type { NameStyle } from '../../ipc/bindings/NameStyle';
import type { UserNote } from '../../ipc/bindings/UserNote';

export interface Who {
  login: string;
  display: string;
}

export interface ShownName {
  name: string;
  /** Shown dimmed after the name, e.g. the login of a localized name. */
  extra?: string;
}

export function isLocalized(w: Who): boolean {
  return w.display.toLowerCase() !== w.login.toLowerCase();
}

export function shownName(w: Who, style: NameStyle, note?: UserNote): ShownName {
  if (note?.nickname) return { name: note.nickname };
  switch (style) {
    case 'login':
      return { name: w.login };
    case 'display':
      return { name: w.display };
    case 'both':
      return isLocalized(w) ? { name: w.display, extra: w.login } : { name: w.display };
  }
}
