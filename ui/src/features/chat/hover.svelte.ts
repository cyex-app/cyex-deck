// V107-M5 One pointer listener pair for every chat line (was two per line). It tracks which
// line is under the pointer; only that line draws its hover buttons. Each line listens to
// its own id, so a move from one line to the next wakes those two, not every line drawn
// (a SvelteSet's `has` of a missing id wakes on any change).
import { createSubscriber } from 'svelte/reactivity';

type Slot = { subscribe: () => void; update?: () => void };

const slots = new Map<string, Slot>();
let under: string | null = null;
let armed = false;

function lineId(t: EventTarget | null): string | null {
  return t instanceof Element ? (t.closest('.line[data-id]')?.getAttribute('data-id') ?? null) : null;
}

function set(id: string | null) {
  if (id === under) return;
  const was = under;
  under = id;
  if (was !== null) slots.get(was)?.update?.();
  if (id !== null) slots.get(id)?.update?.();
}

function slot(id: string): Slot {
  let s = slots.get(id);
  if (s) return s;
  const fresh: Slot = {
    subscribe: createSubscriber((update) => {
      fresh.update = update;
      // The last line showing this id left.
      return () => {
        fresh.update = undefined;
        if (slots.get(id) === fresh) slots.delete(id);
      };
    }),
  };
  slots.set(id, fresh);
  return fresh;
}

function arm() {
  if (armed || typeof document === 'undefined') return;
  armed = true;
  document.addEventListener('pointerover', (e) => set(lineId(e.target)), { passive: true });
  // Out of the window: no line is hot.
  document.addEventListener('pointerout', (e) => e.relatedTarget === null && set(null), { passive: true });
}

/** True while the pointer is over the line with this id. */
export function isHot(id: string): boolean {
  arm();
  // Outside an effect nothing would ever let go of the slot.
  if ($effect.tracking()) slot(id).subscribe();
  return under === id;
}
