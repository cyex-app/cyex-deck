// Short notices. Info and success stack bottom right, errors bottom left (same as cyex logs).
// The same text again bumps a count instead of stacking; each side shows at most 3 (the
// oldest goes); a hovered or focused toast waits; errors stay longer. A toast can carry one
// button (Undo).

export interface Toast {
  id: number;
  kind: 'info' | 'success' | 'error';
  text: string;
  /** How many times it came in; shown from 2 up. */
  count: number;
  action?: ToastAction;
}

/** One button on the toast, e.g. Undo after a one-click delete. Using it closes the toast. */
export interface ToastAction {
  label: string;
  run: () => void;
}

export interface ToastOptions {
  action?: ToastAction;
}

export const LIFETIME_MS = { info: 3500, success: 3500, error: 9000 };
/** A toast with a button stays at least this long, so there's time to reach it. */
export const ACTION_MS = 7000;
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

  info(text: string, opts?: ToastOptions): void {
    this.push('info', text, opts);
  }

  /** V105-16 Green edge: something you asked for worked. */
  success(text: string, opts?: ToastOptions): void {
    this.push('success', text, opts);
  }

  error(text: string, opts?: ToastOptions): void {
    this.push('error', text, opts);
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

  /** Uses the toast's button, then closes it. */
  act(id: number): void {
    const t = this.items.find((x) => x.id === id);
    this.dismiss(id);
    t?.action?.run();
  }

  private push(kind: Toast['kind'], text: string, opts: ToastOptions = {}): void {
    const life = opts.action ? Math.max(LIFETIME_MS[kind], ACTION_MS) : LIFETIME_MS[kind];
    const same = this.items.find((t) => t.kind === kind && t.text === text);
    if (same) {
      // The newest button wins: Undo undoes the last one.
      this.items = this.items.map((t) => (t === same ? { ...t, count: t.count + 1, action: opts.action ?? t.action } : t));
      const c = this.clocks.get(same.id);
      if (c && !c.held) this.run(same.id, life);
      else if (c) c.left = life;
      return;
    }
    const id = nextId++;
    let items = [...this.items, { id, kind, text, count: 1, action: opts.action }];
    const mine = items.filter((t) => side(t.kind) === side(kind));
    for (const old of mine.slice(0, Math.max(0, mine.length - MAX_VISIBLE))) {
      clearTimeout(this.clocks.get(old.id)?.timer);
      this.clocks.delete(old.id);
      items = items.filter((t) => t !== old);
    }
    this.items = items;
    this.run(id, life);
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
