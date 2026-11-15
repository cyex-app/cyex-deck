// V111-29 The follower count of the channel highlighted in Add a tab. Asked once per channel for
// the session, only for the one being looked at (never one request per row or keystroke).
import { formatNumber } from '../../lib/format.ts';

type Ask = (login: string) => Promise<number | null>;

export class FollowerCounts {
  readonly #ask: Ask;
  /** Login → answer; a failed ask is dropped so a later look tries again. */
  readonly #seen = new Map<string, Promise<number | null>>();

  constructor(ask: Ask) {
    this.#ask = ask;
  }

  /** The count, or null when Twitch doesn't give one. */
  get(login: string): Promise<number | null> {
    const key = login.toLowerCase();
    let p = this.#seen.get(key);
    if (!p) {
      p = this.#ask(key).catch(() => null);
      this.#seen.set(key, p);
      void p.then((n) => {
        if (n === null && this.#seen.get(key) === p) this.#seen.delete(key);
      });
    }
    return p;
  }
}

/** "12.345 followers", "1 follower". */
export function followersText(n: number): string {
  return `${formatNumber(n)} ${n === 1 ? 'follower' : 'followers'}`;
}
