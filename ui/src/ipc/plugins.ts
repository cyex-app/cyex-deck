import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import type { PluginCompletion } from './bindings/PluginCompletion';
import type { PluginLog } from './bindings/PluginLog';
import type { PluginMenuEntry } from './bindings/PluginMenuEntry';
import type { PluginOpen } from './bindings/PluginOpen';
import type { PluginsView } from './bindings/PluginsView';

/** S5 Lua plugins. */
export const pluginsIpc = {
  view: () => invoke<PluginsView>('plugins_view'),
  /** On agrees to the permissions the plugin asks for now. */
  setOn: (id: string, on: boolean) => invoke<void>('plugins_set_on', { id, on }),
  setValue: (id: string, key: string, value: unknown) => invoke<void>('plugins_set_value', { id, key, value }),
  /** Without an id: every plugin, and folders added since. */
  reload: (id?: string) => invoke<void>('plugins_reload', { id: id ?? null }),
  openFolder: (id?: string) => invoke<void>('plugins_open_folder', { id: id ?? null }),
  logs: (id: string) => invoke<PluginLog[]>('plugins_logs', { id }),
  clearLog: (id: string) => invoke<void>('plugins_clear_log', { id }),
  repl: (id: string, code: string) => invoke<string>('plugins_repl', { id, code }),
  complete: (view: number, query: string, fullText: string, cursor: number, firstWord: boolean) =>
    invoke<PluginCompletion>('plugins_complete', { view, query, fullText, cursor, firstWord }),
  menus: () => invoke<PluginMenuEntry[]>('plugins_menus'),
  runMenu: (view: number, plugin: string, index: number, msgId: string | null) =>
    invoke<void>('plugins_menu_run', { view, plugin, index, msgId }),
  /** S11 A name entry: the chat's channel, who, and the line the name was on (if any). */
  runName: (plugin: string, index: number, channel: string, login: string, view: number | null, msgId: string | null) =>
    invoke<void>('plugins_name_run', { plugin, index, channel, login, view, msgId }),
  /** S13 A plugin asks the main window to show a tab or open a channel. */
  onOpen: (cb: (o: PluginOpen) => void) => listen<PluginOpen>('plugins:open', (e) => cb(e.payload)),
  onChanged: (cb: () => void) => listen('plugins:changed', () => cb()),
  onLog: (cb: (id: string, line: PluginLog) => void) =>
    listen<{ id: string; line: PluginLog }>('plugins:log', (e) => cb(e.payload.id, e.payload.line)),
};
