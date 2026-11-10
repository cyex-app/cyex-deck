// V110-A-idea A copy button shows a check mark for a moment instead of a toast. Pure timer
// part: `show(key)` lights `key`, a later `show` moves it, it goes out after COPIED_MS.

/** How long the check mark stays. */
export const COPIED_MS = 1200;

export interface FlashTimers {
  set: (run: () => void, ms: number) => unknown;
  clear: (id: unknown) => void;
}

const REAL: FlashTimers = {
  set: (run, ms) => setTimeout(run, ms),
  clear: (id) => clearTimeout(id as ReturnType<typeof setTimeout>),
};

export function copyFlash(on: (key: string | null) => void, t: FlashTimers = REAL) {
  let id: unknown = null;
  const stop = () => {
    if (id !== null) t.clear(id);
    id = null;
  };
  return {
    show(key: string) {
      stop();
      on(key);
      id = t.set(() => {
        id = null;
        on(null);
      }, COPIED_MS);
    },
    stop() {
      stop();
      on(null);
    },
  };
}

/** What a screen reader hears while the check shows: "Link copied." */
export function copiedSaid(what: string): string {
  return `${what.charAt(0).toUpperCase()}${what.slice(1)} copied.`;
}
