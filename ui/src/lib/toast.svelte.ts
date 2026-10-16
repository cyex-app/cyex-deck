// Short notices. Info and success stack bottom right, errors bottom left (same as cyex logs).
// The same text again bumps a count instead of stacking; each side shows at most 3 (the
// oldest goes); a hovered or focused toast waits; errors stay longer.

export interface Toast {
  id: number;
  kind: 'info' | 'success' | 'error';
  text: string;
  /** How many times it came in; shown from 2 up. */
  count: number;
}

export const LIFETIME_MS = { info: 3500, success: 3500, error: 9000 };
/** Per side: errors on one, the rest on the other. */
export const MAX_VISIBLE = 3;

const side = (k: Toast['kind']) => (k === 'error' ? 'error' : 'note');

interface Clock {
  timer: ReturnType<typeof setTimeout> | undefined;
  /** Time left; counts down only while not held. */
  left: number;
  since: number;
  held: boolean;
}

let nextId = 0;

export class ToastStore {
  items = $state<Toast[]>([]);
  private clocks = new Map<number, Clock>();

  info(text: string): void {
    this.push('info', text);
  }

  /** V105-16 Green edge: something you asked for worked. */
  success(text: string): void {
    this.push('success', text);
  }

  error(text: string): void {
    this.push('error', text);
  }

  dismiss(id: number): void {
    const c = this.clocks.get(id);
    clearTimeout(c?.timer);
    this.clocks.delete(id);
    this.items = this.items.filter((t) => t.id !== id);
  }

  /** Pointer or focus on it: the timer waits. */
  hold(id: number): void {
    const c = this.clocks.get(id);
    if (!c || c.held) return;
    clearTimeout(c.timer);
    c.left -= Date.now() - c.since;
    c.held = true;
  }

  /** Let go: the rest of its time runs, never less than a second. */
  release(id: number): void {
    const c = this.clocks.get(id);
    if (!c?.held) return;
    c.held = false;
    this.run(id, Math.max(c.left, 1000));
  }

  private push(kind: Toast['kind'], text: string): void {
    const same = this.items.find((t) => t.kind === kind && t.text === text);
    if (same) {
      this.items = this.items.map((t) => (t === same ? { ...t, count: t.count + 1 } : t));
      const c = this.clocks.get(same.id);
      if (c && !c.held) this.run(same.id, LIFETIME_MS[kind]);
      else if (c) c.left = LIFETIME_MS[kind];
      return;
    }
    const id = nextId++;
    let items = [...this.items, { id, kind, text, count: 1 }];
    const mine = items.filter((t) => side(t.kind) === side(kind));
    for (const old of mine.slice(0, Math.max(0, mine.length - MAX_VISIBLE))) {
      clearTimeout(this.clocks.get(old.id)?.timer);
      this.clocks.delete(old.id);
      items = items.filter((t) => t !== old);
    }
    this.items = items;
    this.run(id, LIFETIME_MS[kind]);
  }

  private run(id: number, ms: number): void {
    const c = this.clocks.get(id) ?? { timer: undefined, left: ms, since: 0, held: false };
    clearTimeout(c.timer);
    c.left = ms;
    c.since = Date.now();
    c.timer = setTimeout(() => this.dismiss(id), ms);
    this.clocks.set(id, c);
  }
}

export const toast = new ToastStore();
