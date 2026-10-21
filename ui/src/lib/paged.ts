// V107-R37..R40 One paged list for the stream manager (clips, videos, followers, the
// redemption queue): the first page, "Load more", and a ticked selection. Only the newest
// request counts, so a slow reply for an old filter or an old page can't land in the list
// (BUG-02); a row on two pages shows once; the selection drops rows that left. Plain TS
// writing into a `PagedSink`, so the tests run it on an object and `paged.svelte.ts` on
// reactive state.

import { errorText } from './errors.ts';

export interface PageReply<T> {
  items: T[];
  next: string | null;
  /** How many there are in all, when Twitch says. */
  total?: number | null;
}

/** What the list shows. */
export interface PagedSink<T> {
  items: T[];
  next: string | null;
  total: number | null;
  /** The first page (or a reload) is on its way. */
  loading: boolean;
  /** "Load more" is on its way. */
  more: boolean;
  failed: string;
  picked: ReadonlySet<string>;
}

export interface PagedOptions<T> {
  /** Reads the page after `cursor` (null: the first) with the filters as they are now. */
  fetch: (cursor: string | null) => Promise<PageReply<T>>;
  key: (t: T) => string;
}

export function emptySink<T>(): PagedSink<T> {
  return { items: [], next: null, total: null, loading: false, more: false, failed: '', picked: new Set() };
}

/** `list` with repeats dropped, first one kept. */
export function deduped<T>(list: readonly T[], key: (t: T) => string): T[] {
  const seen = new Set<string>();
  return list.filter((t) => {
    const k = key(t);
    if (seen.has(k)) return false;
    seen.add(k);
    return true;
  });
}

/** One list after "Load more": new rows appended, repeats dropped. */
export function appended<T>(have: readonly T[], more: readonly T[], key: (t: T) => string): T[] {
  const seen = new Set(have.map(key));
  return [...have, ...deduped(more, key).filter((t) => !seen.has(key(t)))];
}

/** Flips one key in a selection. */
export function toggled(sel: ReadonlySet<string>, key: string): Set<string> {
  const next = new Set(sel);
  if (!next.delete(key)) next.add(key);
  return next;
}

/** The selection minus what's no longer listed. */
export function kept(sel: ReadonlySet<string>, listed: readonly string[]): Set<string> {
  const here = new Set(listed);
  return new Set([...sel].filter((k) => here.has(k)));
}

export class Pager<T> {
  private seq = 0;
  private readonly s: PagedSink<T>;
  private readonly o: PagedOptions<T>;

  constructor(sink: PagedSink<T>, options: PagedOptions<T>) {
    this.s = sink;
    this.o = options;
  }

  private keys(): string[] {
    return this.s.items.map(this.o.key);
  }

  private prune(): void {
    if (this.s.picked.size) this.s.picked = kept(this.s.picked, this.keys());
  }

  /** The first page again. `clear`: the filters changed, so what shows goes at once (the
   *  total too); otherwise it stays until the new page is in. True when this reply landed. */
  async reload({ clear = false }: { clear?: boolean } = {}): Promise<boolean> {
    const mine = ++this.seq;
    this.s.loading = true;
    this.s.more = false;
    if (clear) {
      this.s.items = [];
      this.s.next = null;
      this.s.total = null;
      this.s.failed = '';
      this.s.picked = new Set();
    }
    try {
      const p = await this.o.fetch(null);
      if (mine !== this.seq) return false;
      this.s.items = deduped(p.items, this.o.key);
      this.s.next = p.next;
      this.s.total = p.total ?? null;
      this.s.failed = '';
      this.prune();
      return true;
    } catch (e) {
      if (mine === this.seq) this.s.failed = errorText(e);
      return false;
    } finally {
      if (mine === this.seq) this.s.loading = false;
    }
  }

  /** The next page, added below. Not while a page is on its way. */
  async loadMore(): Promise<boolean> {
    const cursor = this.s.next;
    if (!cursor || this.s.loading || this.s.more) return false;
    const mine = ++this.seq;
    this.s.more = true;
    try {
      const p = await this.o.fetch(cursor);
      if (mine !== this.seq) return false;
      this.s.items = appended(this.s.items, p.items, this.o.key);
      this.s.next = p.next;
      if (p.total != null) this.s.total = p.total;
      this.s.failed = '';
      return true;
    } catch (e) {
      if (mine === this.seq) this.s.failed = errorText(e);
      return false;
    } finally {
      if (mine === this.seq) this.s.more = false;
    }
  }

  /** Rows that went (deleted, settled): out of the list, the total and the selection. */
  remove(keys: Iterable<string>): void {
    const gone = new Set(keys);
    const before = this.s.items.length;
    this.s.items = this.s.items.filter((t) => !gone.has(this.o.key(t)));
    const n = before - this.s.items.length;
    if (n && this.s.total != null) this.s.total = Math.max(0, this.s.total - n);
    this.prune();
  }

  /** One row changed in place. */
  patch(key: string, change: (t: T) => T): void {
    this.s.items = this.s.items.map((t) => (this.o.key(t) === key ? change(t) : t));
  }

  toggle(key: string): void {
    this.s.picked = toggled(this.s.picked, key);
  }

  /** Every loaded row, or none. */
  pickAll(on: boolean): void {
    this.s.picked = on ? new Set(this.keys()) : new Set();
  }

  /** The picked rows, in list order. */
  pickedItems(): T[] {
    return this.s.items.filter((t) => this.s.picked.has(this.o.key(t)));
  }
}
