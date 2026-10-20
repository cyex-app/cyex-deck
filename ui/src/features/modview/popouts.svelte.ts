// V107-R29 Which mod view panels are in a window of their own right now, for every mod
// view in this window. Started by the first view that asks.

import { popOut } from '../../app/panels';
import { windowsIpc } from '../../ipc/windows';
import { panelRoute, poppedFor } from './popped';

class ModPopouts {
  routes = $state.raw<string[]>([]);
  /** The open windows are known: what's missing from `routes` is closed. */
  ready = $state(false);
  private started = false;

  start(): void {
    if (this.started) return;
    this.started = true;
    void windowsIpc.onPanels((r) => (this.routes = r));
    void this.refresh();
  }

  async refresh(): Promise<void> {
    try {
      this.routes = await windowsIpc.panels();
      this.ready = true;
    } catch {
      // Heard of with the next change.
    }
  }

  of(login: string): Set<string> {
    return poppedFor(this.routes, login);
  }

  open(login: string, id: string, title: string): Promise<boolean> {
    return popOut(panelRoute(login, id), `${title} · ${login}`);
  }
}

export const modPopouts = new ModPopouts();
