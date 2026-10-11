// 1.35 The stream manager's live state for this window: who runs it, the stats, the
// activity feed and the X4 viewer graph. Rust runs the feed and the poller while any
// window has the manager open.

import type { UnlistenFn } from '@tauri-apps/api/event';

import type { SmActivity } from '../../ipc/bindings/SmActivity';
import type { SmBroadcaster } from '../../ipc/bindings/SmBroadcaster';
import type { SmRaid } from '../../ipc/bindings/SmRaid';
import type { SmStats } from '../../ipc/bindings/SmStats';
import { smIpc } from '../../ipc/streammanager';
import { capPush } from './logic';

const MAX_ACTIVITY = 200;
const MAX_SERIES = 720;

class StreamManagerStore {
  manager = $state<SmBroadcaster | null>(null);
  stats = $state<SmStats | null>(null);
  /** Oldest first, like Rust's buffer. */
  activity = $state<SmActivity[]>([]);
  series = $state<[number, number][]>([]);
  /** D2 The raid counting down from the channel. */
  raid = $state<SmRaid | null>(null);
  ready = $state(false);

  private users = 0;
  private stops: Promise<UnlistenFn>[] = [];

  /** A view opened. The first one starts listening and reads what Rust has. */
  async open(): Promise<void> {
    if (++this.users > 1) return;
    this.stops = [
      smIpc.onStats((s) => {
        this.stats = s;
        if (s.live) this.series = capPush(this.series, [Math.floor(Date.now() / 1000), s.viewers], MAX_SERIES);
      }),
      smIpc.onActivity((a) => (this.activity = capPush(this.activity, a, MAX_ACTIVITY))),
      smIpc.onActivityAll((all) => (this.activity = all)),
      smIpc.onAlert((id, status) => (this.activity = this.activity.map((a) => (a.alert === id ? { ...a, status } : a)))),
      smIpc.onRaid((r) => (this.raid = r)),
    ];
    try {
      const o = await smIpc.open();
      this.manager = o.manager;
      this.activity = o.activity;
      this.stats = o.stats ?? this.stats;
      this.series = o.series;
      this.raid = o.raid;
    } finally {
      this.ready = true;
    }
  }

  close(): void {
    if (--this.users > 0) return;
    this.users = 0;
    for (const s of this.stops) void s.then((f) => f());
    this.stops = [];
    void smIpc.close();
  }
}

export const sm = new StreamManagerStore();
