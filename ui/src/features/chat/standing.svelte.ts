// V108-C40 One chat box's view of its account's standing: read at join and after every
// reconnect, again whenever IRC or the account's topic says something moved, and on a timer
// while it matters (chatLock.ts `nextRead`). Holds the warning until it's acknowledged.

import type { ChatStanding } from '../../ipc/bindings/ChatStanding';
import type { ChatWarning } from '../../ipc/bindings/ChatWarning';
import type { PastUnban } from '../../ipc/bindings/PastUnban';
import { standingIpc } from '../../ipc/standing';
import { errorText } from '../../lib/errors';
import { lockOf, nextRead, type Lock, type LockInput } from './chatLock';
import type { ChatRoom } from './room.svelte';

export class Standing {
  standing = $state.raw<ChatStanding | null>(null);
  readAt = $state(0);
  warning = $state.raw<ChatWarning | null>(null);
  now = $state(Date.now());
  busy = $state(false);
  error = $state('');
  /** The request this box sent, for when Twitch won't show the requester their own. */
  private sent = $state.raw<PastUnban | null>(null);
  private seq = 0;
  /** V108-F57e Set by stop(): a read or a topic message after that does nothing. */
  private dead = false;
  private timer: ReturnType<typeof setTimeout> | undefined;

  constructor(private readonly room: () => ChatRoom) {}

  /** The account and channel this box reads for, or null (anonymous, not joined yet). */
  private who(): { account: string; channelId: string } | null {
    const r = this.room();
    return r.account && r.state.channelId ? { account: r.account, channelId: r.state.channelId } : null;
  }

  private input(): LockInput {
    const s = this.standing;
    const standing = s && !s.request && this.sent && s.ban?.until === null ? { ...s, request: this.sent } : s;
    return { standing, readAt: this.readAt, said: this.room().said, warning: this.warning, now: this.now };
  }

  get lock(): Lock | null {
    return lockOf(this.input());
  }

  async load(): Promise<void> {
    if (this.dead) return;
    const who = this.who();
    if (!who) return;
    const seq = ++this.seq;
    const startedAt = Date.now();
    try {
      const s = await standingIpc.read(who.account, who.channelId);
      if (seq !== this.seq) return;
      this.standing = s;
      this.readAt = startedAt;
    } catch {
      // A failed read keeps what was known; IRC's word still locks.
    }
    if (seq === this.seq) this.plan();
  }

  /** The next read, if the lock wants one. */
  private plan(): void {
    if (this.dead) return;
    clearTimeout(this.timer);
    this.now = Date.now();
    const wait = nextRead(lockOf(this.input()), this.input());
    if (wait !== null) this.timer = setTimeout(() => void this.load(), wait);
  }

  /** Ticks the clock once a second while a countdown shows. */
  tick(): () => void {
    const t = setInterval(() => (this.now = Date.now()), 1000);
    return () => clearInterval(t);
  }

  /** The account's topic said something moved here; a warning comes with its words. */
  changed(channelId: string, account: string, warning: ChatWarning | null): void {
    if (this.dead) return;
    const who = this.who();
    if (!who || who.account !== account || who.channelId !== channelId) return;
    if (warning) this.warning = warning;
    void this.load();
  }

  async acknowledge(): Promise<void> {
    const who = this.who();
    if (!who || this.busy) return;
    this.busy = true;
    this.error = '';
    try {
      await standingIpc.acknowledge(who.account, who.channelId);
      this.warning = null;
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
      const at = new Date().toISOString();
      this.sent = { id: '', createdAt: at, text: message, status: 'PENDING', resolvedAt: null, resolvedBy: null, note: '' };
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
