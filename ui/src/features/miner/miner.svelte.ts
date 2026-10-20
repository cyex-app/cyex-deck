// The miner as the main window sees it (08): its status and log (W4), kept current by
// Rust's events from the first call of `start` on.

import type { LogLine } from '../../ipc/bindings/LogLine';
import type { MinedChannel } from '../../ipc/bindings/MinedChannel';
import type { MinerStatus } from '../../ipc/bindings/MinerStatus';
import { minerIpc } from '../../ipc/miner';

const LOG_MAX = 1000;

class MinerStore {
  status = $state.raw<MinerStatus | null>(null);
  log = $state.raw<LogLine[]>([]);
  /** V107-R8 Looked up when the view opens while the miner is off. */
  peeked = $state.raw<Map<string, MinedChannel>>(new Map());
  private started = false;

  /** 1.33 Channels getting watch time from the miner now. */
  get mining(): string[] {
    return (this.status?.channels ?? []).filter((c) => c.mining).map((c) => c.name);
  }

  /** V107-R8 The view opened: avatars and who's live, if the miner is off. */
  async peek(): Promise<void> {
    if (this.status?.running) return;
    try {
      const list = await minerIpc.peek();
      // Rust returns the cached list plus new channels, so it replaces the map.
      if (list.length) this.peeked = new Map(list.map((c) => [c.id, c]));
    } catch {
      // Rows stay as they are.
    }
  }

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await minerIpc.onStatus((s) => (this.status = s));
    await minerIpc.onLog((l) => {
      if (this.log.length && this.log[this.log.length - 1].id >= l.id) return;
      this.log = [...this.log, l].slice(-LOG_MAX);
    });
    await minerIpc.onLogCleared(() => (this.log = []));
    const [status, log] = await Promise.all([minerIpc.status(), minerIpc.log()]);
    this.status = status;
    this.log = log;
  }
}

export const miner = new MinerStore();
