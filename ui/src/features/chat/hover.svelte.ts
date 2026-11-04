// V107-M5 One pointer listener pair for every chat line (was two per line). It tracks which
// line is under the pointer; only that line draws its hover buttons. Each line listens to
// its own id, so a move from one line to the next wakes those two, not every line drawn
// (a SvelteSet's `has` of a missing id wakes on any change).
import { createSubscriber } from 'svelte/reactivity';

import { Pick, type Owner } from './hoverPick';

type Slot = { subscribe: () => void; update?: () => void };

const slots = new Map<string, Slot>();
let under: string | null = null;
/** V108-F54 The line picked with the keyboard: hot like the one under the pointer. V108-F57d Owned by its list. */
const picked = new Pick();
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

/** V108-F54 Picks a line (or none) for the keyboard; only it and the one before wake. */
export function pick(owner: Owner, id: string | null) {
  const had = picked.holds(owner);
  const changed = picked.set(owner, id);
  for (const x of changed) slots.get(x)?.update?.();
  // V108-F54b The line under the pointer gives way while one is picked, and comes back after.
  if (changed.length && had !== picked.holds(owner) && under !== null) slots.get(under)?.update?.();
}

/** True for the line the keyboard picked in its own list, else the one under the pointer: one bar at a time. */
export function isHot(id: string, owner?: Owner): boolean {
  arm();
  // Outside an effect nothing would ever let go of the slot.
  if ($effect.tracking()) slot(id).subscribe();
  return picked.hot(id, owner, under);
}

/** V108-F60a True only for the keyboard-picked line: it gets a selection look the hover doesn't. */
export function isPicked(id: string, owner?: Owner): boolean {
  if ($effect.tracking()) slot(id).subscribe();
  return picked.picks(id, owner);
}
