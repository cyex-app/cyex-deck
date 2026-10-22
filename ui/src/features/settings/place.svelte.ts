// V107-Mo Where you left Settings, for the rest of this run (not saved): the search text and,
// per page, which parts are open. The page itself lives in App, the scroll in keepScroll.
// Leaving Settings by any way (cog, Ctrl+,, mouse buttons, a tab) unmounts it; coming back
// reads this. Only lasting choices belong here, not dialogs, menus or tooltips.

const kept = new Map<string, boolean>();
const id = (page: string, key: string) => `${page}\u0000${key}`;

export const settingsPlace = {
  query: '',
  /** The page Settings showed last; '' before the first visit. */
  page: '',
  /**
   * Settings opens on `section`. True when that's where you left (search and scroll still
   * fit); another page (a jump from the account menu, say) drops the search.
   */
  arrive(section: string): boolean {
    const back = this.page === section;
    if (!back) this.query = '';
    this.page = section;
    return back;
  },
  /** Is this part of the page open? `fallback` the first time. */
  open(page: string, key: string, fallback = false): boolean {
    return kept.get(id(page, key)) ?? fallback;
  },
  setOpen(page: string, key: string, value: boolean): void {
    kept.set(id(page, key), value);
  },
  /** Drops `page` keys starting with `prefix` whose rest isn't in `alive` (a removed account). */
  prune(page: string, prefix: string, alive: readonly string[]): void {
    const head = id(page, prefix);
    for (const k of [...kept.keys()]) {
      if (k.startsWith(head) && !alive.includes(k.slice(head.length))) kept.delete(k);
    }
  },
  /** For tests. */
  reset(): void {
    this.query = '';
    this.page = '';
    kept.clear();
  },
};
