// V108-C40 One chat box's view of its account's standing: read at join and after every
// reconnect, again whenever IRC or the account's topic says something moved, and on a timer
// while it matters (chatLock.ts `nextRead`).
// V109-F79 The standing itself is kept per account and channel (standingCache.ts here, the
// app's `known` store across windows), not in the box: a remount, a pop-out or a reopened
// chat starts locked, a failed read keeps it, and IRC's word outlives the room it came in.

import type { KnownStanding } from '../../ipc/bindings/KnownStanding';
import { standingIpc } from '../../ipc/standing';
import { errorText } from '../../lib/errors';
import { inputOf, lockOf, nextReadOf, NOTHING_KNOWN, type Lock, type LockInput } from './chatLock';
import type { ChatRoom } from './room.svelte';
import { cached, keep, keyOf } from './standingCache';

export class Standing {
  /** What's known for `key`. */
  known = $state.raw<KnownStanding>(NOTHING_KNOWN);
  private key = $state('');
  now = $state(Date.now());
  busy = $state(false);
  error = $state('');
  private seq = 0;
  /** V108-F57e Set by stop(): a read or a topic message after that does nothing. */
  private dead = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly room: () => ChatRoom) {}

  /** The account and channel this box reads for, or null (anonymous, not joined yet). */
  private who(): { account: string; channelId: string; key: string } | null {
    const r = this.room();
    return r.account && r.state.channelId
      ? { account: r.account, channelId: r.state.channelId, key: keyOf(r.account, r.state.channelId) }
      : null;
  }

  /** What's known for this box's pair now; another pair's kept word until it's read. */
  private current(): KnownStanding {
    const who = this.who();
    if (!who) return NOTHING_KNOWN;
    return who.key === this.key ? this.known : (cached(who.key) ?? NOTHING_KNOWN);
  }

  private input(): LockInput {
    return inputOf(this.current(), this.room().said, this.now);
  }

  get lock(): Lock | null {
    return lockOf(this.input());
  }

  private take(key: string, k: KnownStanding): void {
    keep(key, k);
    if (key === this.key) this.known = k;
  }

  async load(): Promise<void> {
    if (this.dead) return;
    const who = this.who();
    if (!who) return;
    const seq = ++this.seq;
    if (who.key !== this.key) {
      this.key = who.key;
      this.known = cached(who.key) ?? NOTHING_KNOWN;
      // A pop-out or a new window: the app's kept word, before Twitch answers.
      if (!cached(who.key)) {
        try {
          this.take(who.key, await standingIpc.known(who.account, who.channelId));
        } catch {
          // Nothing kept; the read below decides.
        }
        if (seq !== this.seq) return;
      }
    }
    try {
      const k = await standingIpc.read(who.account, who.channelId);
      if (seq !== this.seq) return;
      this.take(who.key, k);
    } catch {
      // A failed read keeps what was known; IRC's word still locks. The app may know more
      // (another window's read, IRC's word from a room this box never had).
      try {
        const k = await standingIpc.known(who.account, who.channelId);
        if (seq !== this.seq) return;
        this.take(who.key, k);
      } catch {
        // Keep what this window has.
      }
    }
    if (seq === this.seq) this.plan();
  }

  /** The next read, if the lock wants one. */
  private plan(): void {
    if (this.dead) return;
    clearTimeout(this.timer);
    this.now = Date.now();
    const wait = nextReadOf(this.current(), this.input());
    if (wait !== null) this.timer = setTimeout(() => void this.load(), wait);
  }

  /** Ticks the clock once a second while a countdown shows. */
  tick(): () => void {
    const t = setInterval(() => (this.now = Date.now()), 1000);
    return () => clearInterval(t);
  }

  /** The account's topic said something moved here (a warning is kept by the app). */
  changed(channelId: string, account: string): void {
    if (this.dead) return;
    const who = this.who();
    if (!who || who.account !== account || who.channelId !== channelId) return;
    void this.load();
  }

  async acknowledge(): Promise<void> {
    const who = this.who();
    if (!who || this.busy) return;
    this.busy = true;
    this.error = '';
    try {
      await standingIpc.acknowledge(who.account, who.channelId);
      const k = this.current();
      this.take(who.key, { ...k, warning: null, said: k.said?.how.kind === 'warned' ? null : k.said });
      const r = this.room();
      if (r.said?.how.kind === 'warned') r.said = null;
    } catch (e) {
      this.error = errorText(e);
    } finally {
      this.busy = false;
    }
  }

  /** True when the request went out. */
  async requestUnban(message: string): Promise<boolean> {
    const who = this.who();
    if (!who || this.busy) return false;
    this.busy = true;
    this.error = '';
    try {
      await standingIpc.requestUnban(who.account, who.channelId, message);
      // The app keeps the request as sent; shown pending at once, here and in every window.
      try {
        this.take(who.key, await standingIpc.known(who.account, who.channelId));
      } catch {
        // The read below brings it.
      }
      void this.load();
      return true;
    } catch (e) {
      this.error = errorText(e);
      return false;
    } finally {
      this.busy = false;
    }
  }

  /** Reads are on again (the box's watch began; stop() ended the one before). */
  start(): void {
    this.dead = false;
  }

  stop(): void {
    this.dead = true;
    clearTimeout(this.timer);
    this.seq++;
  }
}
