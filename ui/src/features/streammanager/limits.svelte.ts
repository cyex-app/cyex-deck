// V107-R41 The limits the dashboard panels show (lengths, counts, delays), read once from
// Rust, which is the only one that checks them. Null until read: fields then take any
// length and Rust's answer says what's wrong.

import type { DashboardLimits } from '../../ipc/bindings/DashboardLimits';
import { smIpc } from '../../ipc/streammanager';

class Limits {
  v = $state.raw<DashboardLimits | null>(null);
  private asked = false;

  /** Reads them the first time a panel asks. */
  load(): void {
    if (this.asked) return;
    this.asked = true;
    smIpc
      .limits()
      .then((l) => (this.v = l))
      .catch(() => (this.asked = false));
  }
}

export const limits = new Limits();
