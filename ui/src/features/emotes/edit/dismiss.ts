// K11 V106-31/32 Whether the user moved on while a menu waited for 7TV.
const KINDS = ['pointerdown', 'keydown', 'wheel', 'blur'] as const;

/** Starts watching `target` (the window). The returned stop() ends it and says whether a press,
 *  Escape, a wheel turn or leaving the window came in between. */
export function dismissWatch(target: EventTarget): () => boolean {
  let gone = false;
  const on = (e: Event) => {
    if (e.type !== 'keydown' || (e as KeyboardEvent).key === 'Escape') gone = true;
  };
  for (const k of KINDS) target.addEventListener(k, on, true);
  return () => {
    for (const k of KINDS) target.removeEventListener(k, on, true);
    return gone;
  };
}
