// Focus for layers (dialogs, popovers, menus): it moves in when the layer opens and goes
// back to where it was when it closes. A trapped scope keeps Tab and Shift+Tab inside.
//
//   $effect(() => box && focusScope(box, { trap: true, initial: firstField }));

const TABBABLE = [
  'a[href]',
  'button:not(:disabled)',
  'input:not(:disabled):not([type="hidden"])',
  'select:not(:disabled)',
  'textarea:not(:disabled)',
  '[tabindex]:not([tabindex="-1"])',
  '[contenteditable="true"]',
].join(',');

/** Fields that take typing; checkboxes and radios don't count. */
const TEXT_FIELD = [
  'input:not(:disabled):not([type="hidden"]):not([type="checkbox"]):not([type="radio"])',
  'textarea:not(:disabled)',
  'select:not(:disabled)',
  '[role="combobox"]:not(:disabled)',
].join(',');

export interface ScopeOptions {
  /** Keep Tab inside (modal dialogs). */
  trap?: boolean;
  /** What gets focus first: an element, a function that finds it, or the node itself (default). */
  initial?: HTMLElement | null | ((node: HTMLElement) => HTMLElement | null | undefined);
  /** Give focus back on close (default true). */
  restore?: boolean;
  /** Where focus goes on close when nothing useful had it on open (it was on the page body). */
  fallback?: () => HTMLElement | null | undefined;
}

/** Whether a `tabindex` attribute leaves the element in the Tab order: `-1` (a roving
 *  tabindex, e.g. the unpicked options of a segmented control) takes it out. */
export function inTabOrder(tabindex: string | null): boolean {
  return tabindex !== '-1';
}

/** V110-F81 Whether an element is a Tab stop: not roved out (-1), drawn, and not inside an `inert`
 *  subtree (a preview that only looks like UI): focus() on it does nothing and the trap swallowed Tab. */
export function isStop(tabindex: string | null, drawn: boolean, inertAbove: boolean): boolean {
  return inTabOrder(tabindex) && drawn && !inertAbove;
}

/** What Tab can reach inside `node`, in order, skipping hidden and inert ones. */
export function tabbables(node: HTMLElement): HTMLElement[] {
  return [...node.querySelectorAll<HTMLElement>(TABBABLE)].filter((el) =>
    isStop(el.getAttribute('tabindex'), el.getClientRects().length > 0, !!el.closest('[inert]')),
  );
}

/** V110-F81 Moves Tab on from `stops[start]`: the first stop that really takes focus wins, so one that
 *  refuses (disabled late, removed) doesn't swallow the key. Returns whether focus moved. */
export function focusFrom(stops: HTMLElement[], start: number, back: boolean): boolean {
  if (start < 0) return false;
  let i = start;
  for (let n = 0; n < stops.length; n++) {
    stops[i].focus();
    if (document.activeElement === stops[i]) return true;
    i = wrapTab(i, stops.length, back);
  }
  return false;
}

/** A dialog's first focus: a `data-autofocus` element, the first text field, the primary
 *  button, or null. */
export function firstField(node: HTMLElement): HTMLElement | null {
  const mark = node.querySelector<HTMLElement>('[data-autofocus]');
  // A wrapper marks its field: the first live input or button inside takes the focus.
  const inner = mark?.querySelector<HTMLElement>('input:not(:disabled), button:not(:disabled)');
  return (
    inner ??
    mark ??
    node.querySelector<HTMLElement>(TEXT_FIELD) ??
    node.querySelector<HTMLElement>('.btn.primary:not(:disabled)')
  );
}

/** Where Tab goes from `at` among `count` stops; wraps at both ends. -1 when nothing to reach. */
export function wrapTab(at: number, count: number, back: boolean): number {
  if (!count) return -1;
  if (at < 0) return back ? count - 1 : 0;
  return (at + (back ? -1 : 1) + count) % count;
}

/** V110-F70 Where Tab goes when focus is on something that is not a stop (a roving nav button at
 *  tabindex -1): `after[i]` says stop i comes after the focused element in document order. The
 *  next or previous stop in document order, wrapping at the ends. */
export function stopAround(after: boolean[], back: boolean): number {
  if (!after.length) return -1;
  if (!back) {
    const i = after.indexOf(true);
    return i < 0 ? 0 : i;
  }
  const i = after.lastIndexOf(false);
  return i < 0 ? after.length - 1 : i;
}

/** The stop Tab or Shift+Tab reaches from `at` among `stops`; works when `at` is not one of them. */
export function tabTarget(stops: HTMLElement[], at: Element | null, back: boolean): number {
  const i = at ? stops.indexOf(at as HTMLElement) : -1;
  if (i >= 0 || !at) return wrapTab(i, stops.length, back);
  return stopAround(
    stops.map((s) => !!(at.compareDocumentPosition(s) & Node.DOCUMENT_POSITION_FOLLOWING)),
    back,
  );
}

/** Moves focus into `node`; returns the cleanup that gives it back. */
export function focusScope(node: HTMLElement, opts: ScopeOptions = {}): () => void {
  const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const pick = typeof opts.initial === 'function' ? opts.initial(node) : opts.initial;
  (pick ?? node).focus({ preventScroll: true });

  const onKey = (e: KeyboardEvent) => {
    if (e.key !== 'Tab' || e.defaultPrevented) return;
    const stops = tabbables(node);
    const next = tabTarget(stops, document.activeElement, e.shiftKey);
    e.preventDefault();
    if (!focusFrom(stops, next, e.shiftKey)) node.focus();
  };
  if (opts.trap) node.addEventListener('keydown', onKey);

  return () => {
    if (opts.trap) node.removeEventListener('keydown', onKey);
    if (opts.restore === false) return;
    // Only when focus is still in the layer or was dropped; a click elsewhere keeps its target.
    const now = document.activeElement;
    if (now && now !== document.body && !node.contains(now)) return;
    const back = before?.isConnected && before !== document.body ? before : opts.fallback?.();
    back?.focus({ preventScroll: true });
  };
}
