// V108-7P The account asked "Also connect 7TV?" right now, if any. Callers take a snapshot
// before a Twitch connection starts and report it once it's added.

import { accounts } from '../../lib/accounts.svelte';
import { asksSevenTv, linksOf, readNotNow, saveNotNow, type Link } from './alsoSevenTv';

export type Before = Map<string, Link[]>;

class AlsoSevenTv {
  /** The account the prompt shows for. */
  asking = $state<string | null>(null);
  private notNow = readNotNow();
  /** 7TV was offered in the sign-in dialog this run: not asked again. */
  private offered = new Set<string>();

  seen(userId: string): void {
    this.offered.add(userId);
  }

  /** Every account's connections before a sign-in; `pages`: accounts with Twitch pages. */
  snapshot(pages: string[]): Before {
    return new Map(accounts.list.map((a) => [a.userId, linksOf(a, pages.includes(a.userId)) ?? []]));
  }

  added(link: Link, userId: string, before: Before): void {
    const a = accounts.list.find((x) => x.userId === userId);
    const on = !!a && a.seventv.consent && a.seventv.signedIn;
    if (asksSevenTv(link, before.get(userId) ?? null, on, this.notNow.includes(userId) || this.offered.has(userId))) this.asking = userId;
  }

  /** "Connect" closes the prompt; the caller starts 7TV's sign-in. */
  take(): string | null {
    const id = this.asking;
    this.asking = null;
    return id;
  }

  later(): void {
    const id = this.take();
    if (!id || this.notNow.includes(id)) return;
    this.notNow = [...this.notNow, id];
    saveNotNow(this.notNow);
  }
}

export const alsoSevenTv = new AlsoSevenTv();
