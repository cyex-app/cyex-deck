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
}

/** Whether a `tabindex` attribute leaves the element in the Tab order: `-1` (a roving
 *  tabindex, e.g. the unpicked options of a segmented control) takes it out. */
export function inTabOrder(tabindex: string | null): boolean {
  return tabindex !== '-1';
}

/** What Tab can reach inside `node`, in order, skipping hidden ones. */
export function tabbables(node: HTMLElement): HTMLElement[] {
  return [...node.querySelectorAll<HTMLElement>(TABBABLE)].filter((el) => inTabOrder(el.getAttribute('tabindex')) && el.getClientRects().length > 0);
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

/** Moves focus into `node`; returns the cleanup that gives it back. */
export function focusScope(node: HTMLElement, opts: ScopeOptions = {}): () => void {
  const before = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const pick = typeof opts.initial === 'function' ? opts.initial(node) : opts.initial;
  (pick ?? node).focus({ preventScroll: true });

  const onKey = (e: KeyboardEvent) => {
    if (e.key !== 'Tab' || e.defaultPrevented) return;
    const stops = tabbables(node);
    const next = wrapTab(stops.indexOf(document.activeElement as HTMLElement), stops.length, e.shiftKey);
    e.preventDefault();
    (stops[next] ?? node).focus();
  };
  if (opts.trap) node.addEventListener('keydown', onKey);

  return () => {
    if (opts.trap) node.removeEventListener('keydown', onKey);
    if (opts.restore === false || !before?.isConnected) return;
    // Only when focus is still in the layer or was dropped; a click elsewhere keeps its target.
    const now = document.activeElement;
    if (!now || now === document.body || node.contains(now)) before.focus({ preventScroll: true });
  };
}
