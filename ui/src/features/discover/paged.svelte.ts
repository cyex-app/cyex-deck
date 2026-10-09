// A list Twitch hands out a page at a time (1.7). Later pages need a GQL sign-in.

import type { Page } from '../../ipc/bindings/Page';
import { accounts } from '../../lib/accounts.svelte';

export class Paged<T extends { id: string }> {
  items = $state.raw<T[]>([]);
  cursor = $state<string | null>(null);
  loading = $state(false);
  error = $state<string | null>(null);
  /** Bumped on every reset, so answers to an older list are dropped. */
  private run = 0;

  constructor(private load: (after: string | null) => Promise<Page<T>>) {}

  /** Signed out, Twitch serves only the first page. */
  get canPage(): boolean {
    return accounts.list.some((a) => a.features.includes('follow'));
  }

  async reset(load?: (after: string | null) => Promise<Page<T>>): Promise<void> {
    if (load) this.load = load;
    const run = ++this.run;
    this.items = [];
    this.cursor = null;
    this.error = null;
    await this.fetch(run, null);
  }

  async more(): Promise<void> {
    if (!this.cursor || this.loading || !this.canPage) return;
    await this.fetch(this.run, this.cursor);
  }

  private async fetch(run: number, after: string | null): Promise<void> {
    this.loading = true;
    try {
      const page = await this.load(after);
      if (run !== this.run) return;
      const seen = new Set(this.items.map((x) => x.id));
      this.items = [...this.items, ...page.items.filter((x) => !seen.has(x.id) && seen.add(x.id))];
      this.cursor = page.cursor;
    } catch (e) {
      if (run !== this.run) return;
      this.error = String(e);
      this.cursor = null;
    } finally {
      if (run === this.run) this.loading = false;
    }
  }
}

/** Calls `fn` when the element scrolls near view, e.g. to load the next page. */
export function nearView(node: HTMLElement, fn: () => void) {
  let callback = fn;
  const io = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && callback(), {
    rootMargin: '600px',
  });
  io.observe(node);
  return {
    update(next: () => void) {
      callback = next;
    },
    destroy() {
      io.disconnect();
    },
  };
}
