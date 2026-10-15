// V1051-02 Optimistic picks: show at once, wait for more, send only the last. Pure, so
// node --test drives it with a fake scheduler.

import type { Kind, Picked, SelectError } from '../../ipc/cosmetics';

export interface Pair {
  paint: string | null;
  badge: string | null;
}
export type SaveStatus = 'idle' | 'saving' | 'saved' | { error: SelectError };
export interface Scheduler {
  set(fn: () => void, ms: number): unknown;
  clear(id: unknown): void;
}
const real: Scheduler = { set: (fn, ms) => setTimeout(fn, ms), clear: (id) => clearTimeout(id as number) };

export const UNDO_MS = 5000;

export class Selection {
  confirmed: Pair = { paint: null, badge: null };
  shown: Pair = { paint: null, badge: null };
  status: SaveStatus = 'idle';
  /** The value before the last saved change; gone after `UNDO_MS`. */
  undoable: { kind: Kind; id: string | null } | null = null;

  private timers: Partial<Record<Kind, unknown>> = {};
  private flying = 0;
  private undoTimer: unknown;
  private seq: Record<Kind, number> = { paint: 0, badge: 0 };

  private send: (kind: Kind, id: string | null) => Promise<Picked>;
  private onstate: () => void;
  private wait: number;
  private sched: Scheduler;

  constructor(
    send: (kind: Kind, id: string | null) => Promise<Picked>,
    onstate: () => void,
    wait = 400,
    sched: Scheduler = real,
  ) {
    this.send = send;
    this.onstate = onstate;
    this.wait = wait;
    this.sched = sched;
  }

  /** The account's real picks; drops anything pending. */
  reset(paint: string | null, badge: string | null) {
    for (const k of ['paint', 'badge'] as Kind[]) {
      if (this.timers[k] !== undefined) this.sched.clear(this.timers[k]);
      this.timers[k] = undefined;
      this.seq[k]++;
    }
    this.confirmed = { paint, badge };
    this.shown = { paint, badge };
    this.status = 'idle';
    this.clearUndo();
    this.onstate();
  }

  pick(kind: Kind, id: string | null) {
    this.shown = { ...this.shown, [kind]: id };
    this.status = 'saving';
    this.clearUndo();
    if (this.timers[kind] !== undefined) this.sched.clear(this.timers[kind]);
    this.timers[kind] = this.sched.set(() => void this.fire(kind), this.wait);
    this.onstate();
  }

  undo() {
    if (!this.undoable) return;
    const { kind, id } = this.undoable;
    this.pick(kind, id);
  }

  private async fire(kind: Kind) {
    this.timers[kind] = undefined;
    const id = this.shown[kind];
    const mine = ++this.seq[kind];
    const before = this.confirmed[kind];
    this.flying++;
    try {
      const r = await this.send(kind, id);
      if (mine !== this.seq[kind]) return;
      if (r.type === 'superseded') return;
      this.confirmed = { ...this.confirmed, [kind]: id };
      if (before !== id && this.timers[kind] === undefined) {
        this.undoable = { kind, id: before };
        this.undoTimer = this.sched.set(() => {
          this.undoable = null;
          if (this.status === 'saved') this.status = 'idle';
          this.onstate();
        }, UNDO_MS);
      }
      this.settle('saved');
    } catch (e) {
      if (mine !== this.seq[kind]) return;
      this.shown = { ...this.shown, [kind]: this.confirmed[kind] };
      const err = (e && typeof e === 'object' && 'type' in e ? e : { type: 'down' }) as SelectError;
      this.status = { error: err };
      this.onstate();
    } finally {
      this.flying--;
    }
  }

  /** `saved` only once nothing else is waiting or on its way. */
  private settle(s: 'saved') {
    const busy = this.timers.paint !== undefined || this.timers.badge !== undefined || this.flying > 1;
    if (!busy) this.status = s;
    this.onstate();
  }

  private clearUndo() {
    if (this.undoTimer !== undefined) this.sched.clear(this.undoTimer);
    this.undoTimer = undefined;
    this.undoable = null;
  }
}
