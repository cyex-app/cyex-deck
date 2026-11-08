// V110-U43 Tabs the app made only to pop a raided channel out. The main window makes them
// and the pop-out's frame (another window, same storage) reads them, so the frame can offer
// a real close next to "Dock back". Losing the list only means a dock-only frame.

const KEY = 'cyex.raidTabs';

function read(): string[] {
  try {
    const v = JSON.parse(localStorage.getItem(KEY) ?? '[]');
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === 'string') : [];
  } catch {
    return [];
  }
}

function write(ids: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(ids.slice(-20)));
  } catch {
    // Private mode or blocked storage: the pop-out just shows Dock back.
  }
}

export const raidTabs = {
  list: read,
  has: (id: string) => read().includes(id),
  add: (id: string) => write([...read().filter((x) => x !== id), id]),
  /** Docked (kept as a tab) or closed. */
  drop: (id: string) => write(read().filter((x) => x !== id)),
  /** V110-F18 Keeps only these marks. */
  keep: (ids: readonly string[]) => write(read().filter((x) => ids.includes(x))),
};

/** V110-F18 The one way a raid pop-out is docked (the frame's Dock back, "Bring it back" in the
 * main window, the tab's menu): the mark goes first, so the tab survives the window closing. */
export function dockRaid(id: string): void {
  raidTabs.drop(id);
}
