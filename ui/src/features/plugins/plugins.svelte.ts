// S5 Plugins as this window sees them: the list for Settings, message menu entries, and
// whether any plugin answers Tab. Read again whenever Rust says something changed.

import type { PluginMenuEntry } from '../../ipc/bindings/PluginMenuEntry';
import type { PluginsView } from '../../ipc/bindings/PluginsView';
import { pluginsIpc } from '../../ipc/plugins';

class PluginStore {
  view = $state.raw<PluginsView | null>(null);
  menus = $state.raw<PluginMenuEntry[]>([]);

  /** Some running plugin offers Tab completion. */
  get completes(): boolean {
    return !!this.view?.list.some((p) => p.status === 'running' && p.exports.events.includes('CompletionRequested'));
  }

  load(): void {
    pluginsIpc
      .view()
      .then((v) => (this.view = v))
      .catch(() => {});
    pluginsIpc
      .menus()
      .then((m) => (this.menus = m))
      .catch(() => {});
  }

  start(): () => void {
    let stop: (() => void) | undefined;
    let closed = false;
    this.load();
    void pluginsIpc
      .onChanged(() => this.load())
      .then((un) => (closed ? un() : (stop = un)));
    return () => {
      closed = true;
      stop?.();
    };
  }
}

export const plugins = new PluginStore();
