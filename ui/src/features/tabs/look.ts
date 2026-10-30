// V108-N19 A tab's own chat look: text size and emote size over Settings. Pure helpers; the
// menu entries are in menus.ts, the stored shape is `TabLook` (Rust: schema/tabs).

import type { TabLook } from '../../ipc/bindings/TabLook';

/** Text sizes offered in the menu, px (Settings allows 9 to 32). */
export const CHAT_SIZES = [11, 12, 13, 14, 16, 18, 20, 24] as const;
/** Emote sizes offered in the menu, percent (Settings allows 50 to 300). */
export const EMOTE_SIZES = [75, 100, 125, 150, 200] as const;

/** True when the tab follows Settings. */
export function isDefaultLook(look: TabLook | undefined): boolean {
  return !look || (look.chatSize == null && look.emoteSize == null);
}

/** The CSS variables a chat of this tab sets; none for what follows Settings. */
export function lookVars(look: TabLook | undefined): Record<string, string> {
  const vars: Record<string, string> = {};
  if (look?.chatSize != null) vars['--chat-size'] = `${look.chatSize}px`;
  if (look?.emoteSize != null) vars['--emote-scale'] = String(look.emoteSize / 100);
  return vars;
}

/** `look` with one value changed; `null` goes back to Settings. */
export function withLook(look: TabLook | undefined, patch: Partial<TabLook>): TabLook {
  return { chatSize: look?.chatSize ?? null, emoteSize: look?.emoteSize ?? null, ...patch };
}
