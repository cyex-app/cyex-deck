/** V110-F88 Where the listeners go: `window`, or a stand-in in tests. */
export interface PointerTarget {
  addEventListener(type: string, f: (e: Event) => void): void;
  removeEventListener(type: string, f: (e: Event) => void): void;
}

/** Follows a press: `onmove` on every pointermove, `onend(e)` once on pointerup, pointercancel or
 *  window blur. Returns the stop, which removes the listeners and is safe to call again (call it when the
 *  component unmounts mid-press). */
export function trackPointer(
  onmove: () => void,
  onend: (e: Event) => void,
  target: PointerTarget = window,
): () => void {
  let live = true;
  const stop = () => {
    if (!live) return;
    live = false;
    target.removeEventListener('pointermove', onmove);
    target.removeEventListener('pointerup', end);
    target.removeEventListener('pointercancel', end);
    target.removeEventListener('blur', end);
  };
  const end = (e: Event) => {
    stop();
    onend(e);
  };
  target.addEventListener('pointermove', onmove);
  target.addEventListener('pointerup', end);
  target.addEventListener('pointercancel', end);
  target.addEventListener('blur', end);
  return stop;
}
