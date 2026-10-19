// L2 Chat churns the page's DOM heap: its freed space only goes back with a full garbage
// collection, which a busy, shown page rarely gets, so the page process crept up for hours.
// Every 10 min, once nobody has touched the window for 30 s, the app asks Rust for one
// (about 20 ms, no long task). Hidden windows already give memory back (D28).

export const TRIM_EVERY_MS = 10 * 60_000;
export const TRIM_IDLE_MS = 30_000;
export const TRIM_RETRY_MS = 30_000;

/** What a tick does: trim now, try again soon (the user is busy) or wait a full round. */
export function trimStep(now: number, lastInput: number, hidden: boolean): 'trim' | 'retry' | 'wait' {
  if (hidden) return 'wait';
  return now - lastInput < TRIM_IDLE_MS ? 'retry' : 'trim';
}

const INPUT = ['pointerdown', 'keydown', 'wheel'] as const;

/** Starts the rounds; `trim` asks for the collection. Returns the stop. */
export function startMemTrim(trim: () => Promise<void>): () => void {
  let lastInput = Date.now();
  const touch = () => (lastInput = Date.now());
  for (const e of INPUT) addEventListener(e, touch, { capture: true, passive: true });
  let timer: ReturnType<typeof setTimeout>;
  const tick = () => {
    const step = trimStep(Date.now(), lastInput, document.hidden);
    if (step === 'trim') {
      const go = () => void trim().catch(() => {});
      if (typeof requestIdleCallback === 'function') requestIdleCallback(go, { timeout: 5000 });
      else go();
    }
    timer = setTimeout(tick, step === 'retry' ? TRIM_RETRY_MS : TRIM_EVERY_MS);
  };
  timer = setTimeout(tick, TRIM_EVERY_MS);
  return () => {
    clearTimeout(timer);
    for (const e of INPUT) removeEventListener(e, touch, { capture: true });
  };
}
