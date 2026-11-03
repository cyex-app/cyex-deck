// V108-C37 The cards in the top right corner: someone went live, a drop came in, a badge was
// earned. Newest on top, at most 3 (the oldest goes), each leaves after about 6 s; a hovered
// or focused card waits, and gets at least a second more when let go. Separate from the
// centre toasts (toast.svelte.ts).

import type { NoticeCard } from '../ipc/bindings/NoticeCard';

export interface Card extends NoticeCard {
  id: number;
}

export const CARD_MS = 6000;
export const MAX_CARDS = 3;
/** A card let go of keeps at least this long. */
export const MIN_LEFT_MS = 1000;

interface Clock {
  timer: ReturnType<typeof setTimeout> | undefined;
  left: number;
  since: number;
  held: boolean;
}

let nextId = 0;

export class CardStore {
  items = $state<Card[]>([]);
  private clocks = new Map<number, Clock>();

  push(card: NoticeCard): number {
    const id = nextId++;
    const items = [{ ...card, id }, ...this.items];
    for (const old of items.slice(MAX_CARDS)) this.forget(old.id);
    this.items = items.slice(0, MAX_CARDS);
    this.run(id, CARD_MS);
    return id;
  }

  dismiss(id: number): void {
    this.forget(id);
    this.items = this.items.filter((c) => c.id !== id);
  }

  hold(id: number): void {
    const c = this.clocks.get(id);
    if (!c || c.held) return;
    clearTimeout(c.timer);
    c.left -= Date.now() - c.since;
    c.held = true;
  }

  release(id: number): void {
    const c = this.clocks.get(id);
    if (!c?.held) return;
    c.held = false;
    this.run(id, Math.max(c.left, MIN_LEFT_MS));
  }

  private forget(id: number): void {
    clearTimeout(this.clocks.get(id)?.timer);
    this.clocks.delete(id);
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

export const cards = new CardStore();
