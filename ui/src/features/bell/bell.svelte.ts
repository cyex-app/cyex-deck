// The bell (1.26): the unseen count from Rust, and the lists while the bell is open.

import type { BellItem } from '../../ipc/bindings/BellItem';
import type { BellList } from '../../ipc/bindings/BellList';
import { bellIpc, type BellLive } from '../../ipc/notify';
import { toast } from '../../lib/toast.svelte';

class BellStore {
  available = $state(false);
  unseen = $state(0);
  list = $state<BellList>('viewer');
  items = $state.raw<BellItem[]>([]);
  cursor = $state<string | null>(null);
  loading = $state(false);
  private started = false;

  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await bellIpc.onUnseen((s) => {
      this.available = s.available;
      this.unseen = s.unseen;
    });
    await bellIpc.onLive((l) => this.apply(l));
    const s = await bellIpc.state();
    this.available = s.available;
    this.unseen = s.unseen;
  }

  async load(list: BellList = this.list, more = false): Promise<void> {
    if (this.loading) return;
    this.loading = true;
    this.list = list;
    try {
      const page = await bellIpc.list(list, more ? this.cursor : null);
      this.items = more ? [...this.items, ...page.items] : page.items;
      this.cursor = page.cursor;
    } catch (e) {
      toast.error(String(e));
    } finally {
      this.loading = false;
    }
  }

  /** The bell opened: load the list and clear the badge. */
  async opened(): Promise<void> {
    await this.load(this.list);
    if (this.unseen > 0) bellIpc.seen().catch(() => {});
  }

  async read(item: BellItem): Promise<void> {
    if (item.read) return;
    this.items = this.items.map((i) => (i.id === item.id ? { ...i, read: true } : i));
    bellIpc.read([item.id]).catch(() => {});
  }

  async readAll(): Promise<void> {
    this.items = this.items.map((i) => ({ ...i, read: true }));
    await bellIpc.readAll(this.list).catch((e) => toast.error(String(e)));
  }

  async remove(id: string): Promise<void> {
    this.items = this.items.filter((i) => i.id !== id);
    await bellIpc.remove(id).catch((e) => toast.error(String(e)));
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
