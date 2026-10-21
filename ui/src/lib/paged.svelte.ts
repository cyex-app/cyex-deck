// V107-R37..R40 The paged list as reactive state for a component (see paged.ts).

import { type PagedOptions, type PagedSink, Pager } from './paged.ts';

export class PagedList<T> implements PagedSink<T> {
  items = $state.raw<T[]>([]);
  next = $state<string | null>(null);
  total = $state<number | null>(null);
  /** True from the start: every list reloads when it mounts, so it never flashes empty. */
  loading = $state(true);
  more = $state(false);
  failed = $state('');
  picked = $state.raw<ReadonlySet<string>>(new Set());
  private readonly pager: Pager<T>;

  constructor(options: PagedOptions<T>) {
    this.pager = new Pager<T>(this, options);
  }

  reload = (opts?: { clear?: boolean }) => this.pager.reload(opts);
  loadMore = () => this.pager.loadMore();
  remove = (keys: Iterable<string>) => this.pager.remove(keys);
  patch = (key: string, change: (t: T) => T) => this.pager.patch(key, change);
  toggle = (key: string) => this.pager.toggle(key);
  pickAll = (on: boolean) => this.pager.pickAll(on);
  pickedItems = () => this.pager.pickedItems();
}
