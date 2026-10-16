// The bell (1.26): the unseen count from Rust, and the lists while the bell is open.

import type { BellItem } from '../../ipc/bindings/BellItem';
import type { BellList } from '../../ipc/bindings/BellList';
import { bellIpc, type BellLive } from '../../ipc/notify';
import { latest } from '../../lib/latest';
import { toast } from '../../lib/toast.svelte';

class BellStore {
  available = $state(false);
  unseen = $state(0);
  list = $state<BellList>('viewer');
  items = $state.raw<BellItem[]>([]);
  cursor = $state<string | null>(null);
  loading = $state(false);
  private started = false;
  private tries = 0;

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    try {
      let evented = false;
      await bellIpc.onUnseen((s) => {
        evented = true;
        this.available = s.available;
        this.unseen = s.unseen;
      });
      await bellIpc.onLive((l) => this.apply(l));
      const s = await bellIpc.state();
      if (evented) return;
      this.available = s.available;
      this.unseen = s.unseen;
    } catch {
      // The bell stays hidden until it starts: try again a few times.
      this.started = false;
      if (++this.tries <= 3) setTimeout(() => void this.start(), 5000 * this.tries);
    }
  }

  private seq = latest();

  async load(list: BellList = this.list, more = false): Promise<void> {
    // More pages wait for the one in flight; a new tab replaces it.
    if (this.loading && more) return;
    const live = this.seq.begin();
    this.loading = true;
    if (list !== this.list) {
      this.items = [];
      this.cursor = null;
    }
    this.list = list;
    try {
      const page = await bellIpc.list(list, more ? this.cursor : null);
      if (!live()) return;
      this.items = more ? [...this.items, ...page.items] : page.items;
      this.cursor = page.cursor;
    } catch (e) {
      if (live()) toast.error(String(e));
    } finally {
      if (live()) this.loading = false;
    }
  }

  /** The bell opened: load the list and clear the badge. */
  async opened(): Promise<void> {
    await this.load(this.list);
    if (this.unseen > 0) bellIpc.seen().catch(() => {});
  }

  async read(item: BellItem): Promise<void> {
    if (item.read) return;
    const before = this.items;
    this.items = this.items.map((i) => (i.id === item.id ? { ...i, read: true } : i));
    await bellIpc.read([item.id]).catch(() => this.restore(before, item.id));
  }

  /** An optimistic change failed: put the old items back, keeping what changed since. */
  private restore(before: BellItem[], id?: string): void {
    const old = new Map(before.map((i) => [i.id, i]));
    this.items = this.items.map((i) => (id === undefined || i.id === id ? (old.get(i.id) ?? i) : i));
    toast.error('Couldn’t update the bell.');
  }

  async readAll(): Promise<void> {
    const before = this.items;
    this.items = this.items.map((i) => ({ ...i, read: true }));
    await bellIpc.readAll(this.list).catch(() => this.restore(before));
  }

  async remove(id: string): Promise<void> {
    const before = this.items;
    this.items = this.items.filter((i) => i.id !== id);
    await bellIpc.remove(id).catch(() => {
      // Back where it was.
      this.items = before.filter((i) => i.id === id || this.items.some((x) => x.id === i.id));
      toast.error('Couldn’t remove it.');
    });
  }

  private apply(l: BellLive): void {
    switch (l.kind) {
      case 'created':
        if (l.list === this.list && !this.items.some((i) => i.id === l.item.id)) this.items = [l.item, ...this.items];
        break;
      case 'updated':
        this.items = this.items.map((i) => (i.id === l.item.id ? l.item : i));
        break;
      case 'deleted':
        this.items = this.items.filter((i) => !l.ids.includes(i.id));
        break;
      case 'read':
        this.items = this.items.map((i) => (l.ids.includes(i.id) ? { ...i, read: true } : i));
        break;
      case 'allRead':
        this.items = this.items.map((i) => ({ ...i, read: true }));
        break;
    }
  }
}

export const bell = new BellStore();
