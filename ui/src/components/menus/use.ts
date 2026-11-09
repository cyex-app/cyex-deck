// V110-U66 What a menu's call site needs to open a registry menu: the user's layout for it,
// the custom entries and what they do. Not pure (settings, IPC); the defs are.

import type { MenuContext } from '../../ipc/bindings/MenuContext';
import type { UiAction } from '../../ipc/bindings/UiAction';
import { chatIpc } from '../../ipc/chat';
import { systemIpc } from '../../ipc/system';
import { copy } from '../../lib/clipboard';
import { errorText } from '../../lib/errors';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import type { MenuEntry } from '../menuItems';
import { customEntry, kindsFor, type Runners, type Values } from './custom.ts';
import { resolveMenu } from './resolve.ts';
import type { MenuDef } from './types.ts';

/** What custom entries may do in this menu. */
export interface Hooks {
  /** The chat view: "Run in chat" needs it. */
  view?: number | null;
  /** Context handed to Rust with a chat command. */
  context?: MenuContext;
  /** What a command's answer asks the window to do. */
  onaction?: (a: UiAction) => void;
  /** Puts text in the message box: "Put in message box" needs it. */
  prefill?: (text: string) => void;
  /** Placeholder values for custom entries. */
  values?: Values;
}

/** The runners for custom entries, from what the menu has. */
export function runnersFor(h: Hooks): Runners {
  return {
    openLink: (url) => void systemIpc.openLink(url).catch((e) => toast.error(errorText(e))),
    chat: (command) => {
      if (h.view == null || !h.context) return;
      chatIpc
        .menu(h.view, command, h.context)
        .then((a) => a && h.onaction?.(a))
        .catch((e) => toast.error(errorText(e)));
    },
    prefill: (text) => h.prefill?.(text),
    copy: (text) => void copy(text, 'Text'),
  };
}

/** The entries to open `def` with: the user's layout if there is one, else the built-in menu. */
export function menuEntries<C>(def: MenuDef<C>, ctx: C, hooks: Hooks = {}): MenuEntry[] {
  const run = runnersFor(hooks);
  const kinds = kindsFor(def.id);
  return resolveMenu(def, settings.v.menus?.layouts?.[def.id], ctx, {
    custom: (node) => {
      const kind = node.action.kind;
      if (!kinds.includes(kind)) return null;
      if (kind === 'chat' && (hooks.view == null || !hooks.context)) return null;
      if (kind === 'prefill' && !hooks.prefill) return null;
      return customEntry(node, hooks.values ?? {}, run);
    },
  });
}
