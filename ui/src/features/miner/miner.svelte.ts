// The miner as the main window sees it (08): its status and log (W4), kept current by
// Rust's events from the first call of `start` on.

import type { LogLine } from '../../ipc/bindings/LogLine';
import type { MinerStatus } from '../../ipc/bindings/MinerStatus';
import { minerIpc } from '../../ipc/miner';

const LOG_MAX = 1000;

class MinerStore {
  status = $state.raw<MinerStatus | null>(null);
  log = $state.raw<LogLine[]>([]);
  private started = false;

  /** 1.33 Channels getting watch time from the miner now. */
  get mining(): string[] {
    return (this.status?.channels ?? []).filter((c) => c.mining).map((c) => c.name);
  }

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await minerIpc.onStatus((s) => (this.status = s));
    await minerIpc.onLog((l) => (this.log = [...this.log, l].slice(-LOG_MAX)));
    await minerIpc.onLogCleared(() => (this.log = []));
    const [status, log] = await Promise.all([minerIpc.status(), minerIpc.log()]);
    this.status = status;
    this.log = log;
  }
}

export const miner = new MinerStore();
