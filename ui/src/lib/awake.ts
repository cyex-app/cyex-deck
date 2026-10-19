// V107-H1 Hidden is lighter: a window that's minimized, in the tray or covered skips
// its repeating work and catches up once when it shows again.

/** What the timers read: `document`, or a stand-in in tests. */
export interface Page {
  readonly hidden: boolean;
  addEventListener(type: 'visibilitychange', f: () => void): void;
  removeEventListener(type: 'visibilitychange', f: () => void): void;
}

const page = (): Page | null => (typeof document === 'undefined' ? null : document);

/** Runs `f` every `ms` while the page shows. Ticks while hidden are skipped; if one was,
 *  `f` runs once as soon as the page shows again. Returns the stop. */
export function everyShown(f: () => void, ms: number, doc: Page | null = page()): () => void {
  let missed = false;
  const timer = setInterval(() => {
    if (doc?.hidden) missed = true;
    else f();
  }, ms);
  const vis = () => {
    if (doc?.hidden || !missed) return;
    missed = false;
    f();
  };
  doc?.addEventListener('visibilitychange', vis);
  return () => {
    clearInterval(timer);
    doc?.removeEventListener('visibilitychange', vis);
  };
}

/** Calls `f(hidden)` on every visibility change. Returns the stop. */
export function onShown(f: (hidden: boolean) => void, doc: Page | null = page()): () => void {
  if (!doc) return () => {};
  const vis = () => f(doc.hidden);
  doc.addEventListener('visibilitychange', vis);
  return () => doc.removeEventListener('visibilitychange', vis);
}

/** True while the page is hidden. */
export function pageHidden(doc: Page | null = page()): boolean {
  return !!doc?.hidden;
}
