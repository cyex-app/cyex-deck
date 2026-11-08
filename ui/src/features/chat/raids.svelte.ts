// V110-U43 Raids counting down from the channels this window's chats show, by channel id.
// Each chat follows its channel's topic while it's open (RaidBanner); one listener here
// keeps the list, so the pin and every chat of the channel see the same raid.

import { SvelteMap } from 'svelte/reactivity';

import type { SmRaid } from '../../ipc/bindings/SmRaid';
import { raidIpc } from '../../ipc/raid';
import { nowSecond } from '../../lib/clock.svelte';

/** A raid whose go message was missed doesn't hang around: gone this long after its time. */
const STALE_S = 30;

class RaidStore {
  private readonly byChannel = new SvelteMap<string, SmRaid>();
  private started = false;
  /** Open chats per channel, in this window. */
  private readonly counts = new Map<string, number>();

  /** The raid counting down from the channel; re-read every second by the caller's scope. */
  of(channelId: string | null): SmRaid | null {
    const r = channelId ? this.byChannel.get(channelId) : undefined;
    return r && nowSecond() / 1000 < r.goesAt + STALE_S ? r : null;
  }

  /** Listens from the first chat on; later calls do nothing. */
  start(): void {
    if (this.started) return;
    this.started = true;
    raidIpc
      .onChange((e) => this.counts.has(e.channelId) && this.put(e.channelId, e.raid))
      .catch(() => {
        this.started = false;
      });
  }

  /** A chat of the channel opened: its topic is followed (counted in Rust) and a raid
   * already counting down is read once. Returns the release for when it closes. */
  follow(channelId: string): () => void {
    this.start();
    this.counts.set(channelId, (this.counts.get(channelId) ?? 0) + 1);
    void raidIpc.watch(channelId, true).catch(() => {});
    raidIpc
      .read(channelId)
      .then((r) => r && !this.byChannel.has(channelId) && this.put(channelId, r))
      .catch(() => {});
    let done = false;
    return () => {
      if (done) return;
      done = true;
      void raidIpc.watch(channelId, false).catch(() => {});
      const n = (this.counts.get(channelId) ?? 1) - 1;
      if (n > 0) this.counts.set(channelId, n);
      else {
        this.counts.delete(channelId);
        this.byChannel.delete(channelId);
      }
    };
  }

  private put(channelId: string, raid: SmRaid | null): void {
    if (raid) this.byChannel.set(channelId, raid);
    else this.byChannel.delete(channelId);
  }
}

export const raids = new RaidStore();
