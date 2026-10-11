// S5, S9 Plugins' menu entries as menu items: on lines (for the kinds they ask for) or in
// the chat's ⋯ menu, entries with a group in a submenu of that name.

import type { MenuEntry, MenuItem } from '../../components/menuItems';
import type { ChatMessage } from '../../ipc/bindings/ChatMessage';
import type { PluginMenuEntry } from '../../ipc/bindings/PluginMenuEntry';
import type { PluginMenuPlace } from '../../ipc/bindings/PluginMenuPlace';

/** The kind of line a message entry can ask for. */
export function lineKind(m: ChatMessage): string {
  if (m.plugin) return 'plugin';
  if (m.notice) return 'notice';
  return m.flags.includes('action') ? 'action' : 'chat';
}

/** The entries for one menu. `kind` is the line's, for message entries. */
export function pluginItems(
  menus: PluginMenuEntry[],
  place: PluginMenuPlace,
  kind: string | null,
  run: (e: PluginMenuEntry) => void,
): MenuEntry[] {
  const out: MenuEntry[] = [];
  const groups = new Map<string, MenuItem & { items: MenuEntry[] }>();
  for (const m of menus) {
    if (m.place !== place || (kind && m.lines.length && !m.lines.includes(kind))) continue;
    const item: MenuItem = { label: m.label, icon: 'puzzle', run: () => run(m) };
    if (!m.group) {
      out.push(item);
      continue;
    }
    let g = groups.get(m.group);
    if (!g) {
      g = { label: m.group, icon: 'puzzle', items: [] };
      groups.set(m.group, g);
      out.push(g);
    }
    g.items.push(item);
  }
  return out;
}
