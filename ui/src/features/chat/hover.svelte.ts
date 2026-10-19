// V107-M5 One pointer listener pair for every chat line (was two per line). It tracks which
// line is under the pointer; only that line draws its hover buttons.
import { SvelteSet } from 'svelte/reactivity';

const under = new SvelteSet<string>();
let armed = false;

function lineId(t: EventTarget | null): string | null {
  return t instanceof Element ? (t.closest('.line[data-id]')?.getAttribute('data-id') ?? null) : null;
}

function set(id: string | null) {
  if (id !== null && under.size === 1 && under.has(id)) return;
  under.clear();
  if (id !== null) under.add(id);
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
  return under.has(id);
}
