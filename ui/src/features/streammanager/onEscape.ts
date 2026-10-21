// V107-R36..R39 Escape backs out of an inline form or confirm in a stream manager panel
// (`use:onEscape={cancel}`), unless something inside used the key first (an open
// dropdown, a search box clearing itself). `focusIn` moves focus in on mount; `focusBack`
// puts it back on the control that opened it once the confirm is gone.

import { tick } from 'svelte';

export function onEscape(node: HTMLElement, cancel: () => void) {
  let run = cancel;
  const key = (e: KeyboardEvent) => {
    if (e.key !== 'Escape' || e.defaultPrevented || e.isComposing) return;
    e.preventDefault();
    e.stopPropagation();
    run();
  };
  node.addEventListener('keydown', key);
  return {
    update: (next: () => void) => (run = next),
    destroy: () => node.removeEventListener('keydown', key),
  };
}

/** Focus on mount: the first field of a form that just opened, the chip that asks, or
 *  what `pick` names inside (an inline confirm starts on Cancel, not on Delete). */
export function focusIn(node: HTMLElement, pick = 'input, button, textarea') {
  const el = node.matches(pick) ? node : node.querySelector<HTMLElement>(pick);
  el?.focus();
}

/** After an inline confirm closes: focus on what opened it (`[data-back="…"]`, or a button inside). */
export function focusBack(key: string) {
  void tick().then(() => {
    const el = document.querySelector<HTMLElement>(`[data-back="${CSS.escape(key)}"]`);
    (el?.matches('button') ? el : el?.querySelector<HTMLElement>('button'))?.focus();
  });
}
