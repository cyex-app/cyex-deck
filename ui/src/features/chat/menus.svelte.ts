// S1 The user's own menu entries and usercard buttons, read from Rust and read again
// when the commands settings change. Picking one runs its command in Rust.

import type { MenuEntry as Item } from '../../components/menu.svelte';
import type { MenuContext } from '../../ipc/bindings/MenuContext';
import type { MenuEntry } from '../../ipc/bindings/MenuEntry';
import type { Menus } from '../../ipc/bindings/Menus';
import type { UiAction } from '../../ipc/bindings/UiAction';
import { chatIpc } from '../../ipc/chat';
import { rulesIpc } from '../../ipc/rules';
import { settings } from '../../lib/settings.svelte';
import { toast } from '../../lib/toast.svelte';
import { errorText } from '../../lib/errors';

const EMPTY: Menus = { user: [], message: [], channel: [], usercard: [] };

class UserMenus {
  current = $state.raw<Menus>(EMPTY);
  private seen: unknown = null;

  /** Keeps the menus in step with the settings; call from a component's effect. */
  track(): void {
    const c = settings.v.commands;
    if (c === this.seen) return;
    this.seen = c;
    rulesIpc
      .menus()
      .then((m) => (this.current = m))
      .catch(() => {});
  }
}

export const userMenus = new UserMenus();

/** Runs a picked entry; the window acts on what comes back. */
export function runEntry(view: number, command: string, ctx: MenuContext, onAction?: (a: UiAction) => void): void {
  chatIpc
    .menu(view, command, ctx)
    .then((a) => a && onAction?.(a))
    .catch((e) => toast.error(errorText(e)));
}

/** The entries as menu items. */
export function toItems(entries: MenuEntry[], run: (command: string) => void): Item[] {
  return entries.map((e): Item => {
    switch (e.t) {
      case 'separator':
        return 'separator';
      case 'sub':
        return { label: e.label, icon: 'list', items: toItems(e.items, run) };
      case 'item':
        return { label: e.label, icon: 'terminal', run: () => run(e.command) };
    }
  });
}
