// Ctrl+F belongs to the app: Settings search, chat search, nothing elsewhere. WebView2's own
// find bar (top right) never opens. The key is stopped early; `keyTaken` ignores that stop.

interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
}

/** Plain Ctrl+F (Shift+Ctrl+F is its own hotkey). */
export function isFindKey(e: KeyLike): boolean {
  return (e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'f';
}

const ours = new WeakSet<object>();

/** True when `e.defaultPrevented` only comes from `blockBrowserFind`. */
export function stoppedByUs(e: object): boolean {
  return ours.has(e);
}

/** Capture-phase handler: stops the browser's find bar and marks the event as ours. */
export function blockBrowserFind(e: KeyLike & { defaultPrevented: boolean; preventDefault(): void }): void {
  if (!isFindKey(e) || e.defaultPrevented) return;
  e.preventDefault();
  ours.add(e);
}

/** Settings: the search box that is showing (the side one or, in narrow windows, the top one). */
export function visibleSearchbox(root: ParentNode): HTMLInputElement | null {
  const all = root.querySelectorAll<HTMLInputElement>('input[role="searchbox"]');
  for (const el of all) if (el.offsetParent !== null) return el;
  return null;
}

let installed = false;

export function installFindBlock(): void {
  if (installed || typeof window === 'undefined') return;
  installed = true;
  window.addEventListener('keydown', blockBrowserFind, true);
}
