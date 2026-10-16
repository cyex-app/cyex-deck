// Home's data (1.6, H2): the picked account's follows and the starred channels, refreshed
// every minute while home shows. The switcher reads the follows too.

import type { ChannelCard } from '../../ipc/bindings/ChannelCard';
import { homeIpc } from '../../ipc/browse';
import { accounts } from '../../lib/accounts.svelte';
import { latest } from '../../lib/latest';
import { settings } from '../../lib/settings.svelte';

const POLL_MS = 60_000;

class HomeStore {
  /** Account picked in home's header; `null` follows the default. */
  picked = $state<string | null>(null);
  follows = $state.raw<ChannelCard[] | null>(null);
  favorites = $state.raw<ChannelCard[]>([]);
  error = $state<string | null>(null);
  loading = $state(false);
  /** Whose follows `follows` holds. */
  private owner: string | null = null;

  /** Accounts that can list follows (1.54). */
  get able(): string[] {
    return accounts.list.filter((a) => a.features.includes('followedList')).map((a) => a.userId);
  }

  /** The account whose follows show: the picked one, else the default, else any able one. */
  get account(): string | null {
    const able = this.able;
    if (this.picked && able.includes(this.picked)) return this.picked;
    const def = accounts.current?.userId;
    return def && able.includes(def) ? def : (able[0] ?? null);
  }

  private refreshes = latest();
  private favSeq = latest();

  async refresh(force = false): Promise<void> {
    const account = this.account;
    const ids = settings.v.browse.favorites.map((c) => c.id);
    // A newer refresh or a favorites reload makes this one's answer stale.
    const live = this.refreshes.begin();
    const favLive = this.favSeq.begin();
    this.loading = true;
    const [follows, favorites] = await Promise.allSettled([
      account ? homeIpc.follows(account, force) : Promise.resolve(null),
      homeIpc.channels(ids),
    ]);
    if (!live()) return;
    this.loading = false;
    if (favorites.status === 'fulfilled' && favLive()) this.favorites = favorites.value;
    if (this.account !== account) return;
    if (follows.status === 'fulfilled') {
      this.follows = follows.value;
      this.owner = account;
      this.error = null;
    } else {
      if (this.owner !== account) this.follows = null;
      this.error = String(follows.reason);
    }
  }

  /** Loads once if nothing is loaded yet, e.g. for the switcher. */
  ensure(): void {
    if (!this.loading && this.owner !== this.account) void this.refresh();
  }

  /** Refreshes now and every minute until the returned stop is called. */
  watch(): () => void {
    void this.refresh();
    const timer = setInterval(() => void this.refresh(), POLL_MS);
    return () => clearInterval(timer);
  }

  /** Favorites changed: fetch just those. */
  async reloadFavorites(): Promise<void> {
    const ids = settings.v.browse.favorites.map((c) => c.id);
    const live = this.favSeq.begin();
    try {
      const f = await homeIpc.channels(ids);
      if (live()) this.favorites = f;
    } catch {
      // Kept as they were; the next refresh tries again.
    }
  }
}

export const home = new HomeStore();
