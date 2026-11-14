// V111-22 When the 7TV profile picture fails or is slow, Twitch's takes its place (and the
// corner logo follows). Pure decision logic; the browser side is in avatarPick.svelte.ts.

/** How long the 7TV picture gets before Twitch's shows. */
export const AVATAR_TIMEOUT_MS = 1500;
/** Logins remembered as failed for the session. */
export const FAILED_CAP = 200;

export type Phase = 'trying' | 'loaded' | 'failed';
export type Event = 'load' | 'error' | 'timeout';

/** `loaded` and `failed` are final: a late load never flips a fallback back (no flicker). */
export function next(phase: Phase, event: Event): Phase {
  if (phase !== 'trying') return phase;
  return event === 'load' ? 'loaded' : 'failed';
}

/** Failed logins, oldest dropped past the cap. */
export class FailedSet {
  private set = new Set<string>();
  private cap: number;
  constructor(cap = FAILED_CAP) {
    this.cap = cap;
  }
  has(login: string): boolean {
    return this.set.has(login);
  }
  add(login: string): void {
    this.set.delete(login);
    this.set.add(login);
    if (this.set.size > this.cap) this.set.delete(this.set.values().next().value as string);
  }
  get size(): number {
    return this.set.size;
  }
}
