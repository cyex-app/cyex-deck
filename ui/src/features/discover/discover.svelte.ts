// Where discover is (1.7). Kept here so it stays put while other views show, and so home
// can send a category or tag over (H4).

import type { StreamSort } from '../../ipc/bindings/StreamSort';

export type Place =
  | { kind: 'streams' }
  | { kind: 'categories' }
  | { kind: 'category'; slug: string; name: string }
  | { kind: 'search'; text: string };

class DiscoverStore {
  place = $state<Place>({ kind: 'streams' });
  /** H4 Streams with this tag only. */
  tag = $state<string | null>(null);
  sort = $state<StreamSort>('viewers');
  private trail = $state<Place[]>([]);

  go(place: Place): void {
    if (place.kind === 'search' && this.place.kind === 'search') {
      this.place = place;
      return;
    }
    this.trail = [...this.trail.slice(-20), this.place];
    this.place = place;
  }

  get canGoBack(): boolean {
    return this.trail.length > 0;
  }

  back(): void {
    const last = this.trail.at(-1);
    if (!last) return;
    this.trail = this.trail.slice(0, -1);
    this.place = last;
  }

  /** H4 */
  showTag(tag: string): void {
    this.tag = tag;
    if (this.place.kind !== 'streams' && this.place.kind !== 'category') this.go({ kind: 'streams' });
  }
}

export const discover = new DiscoverStore();
