// V107-H15 Switching to a tab puts the cursor in its chat input, so you can type at once.
// Not when a dialog or menu is open, nor when the switch came from typing in some other
// field (a search box, a setting). A chat input handing over to the next one is fine.

/** What the page looks like when a switch starts or ends. */
export interface FocusSnapshot {
  /** The focused element is a text field that isn't a chat input. */
  otherField: boolean;
  /** A dialog, menu or list popup is open. */
  layerOpen: boolean;
}

export function mayFocusChat(s: FocusSnapshot): boolean {
  return !s.otherField && !s.layerOpen;
}

const FIELD = 'input, textarea, select, [contenteditable=""], [contenteditable="true"]';
const LAYER = '[role="dialog"], [role="menu"], [role="listbox"]';
const CHAT_INPUT = '.compose textarea';
/** Inputs that take no typing. */
const NON_TEXT = new Set(['button', 'checkbox', 'radio', 'range', 'color', 'file', 'submit', 'reset', 'image']);

/** The focused element is a field the user may be typing in, other than a chat input. */
export function inOtherField(el: Element | null): boolean {
  if (!el || !el.matches(FIELD)) return false;
  if (el.matches(CHAT_INPUT)) return false;
  // The quick switcher's own search box hands over to the tab it picked.
  if (el.closest('[aria-label="Quick switcher"]')) return false;
  if (el.tagName === 'INPUT' && NON_TEXT.has((el as HTMLInputElement).type)) return false;
  return true;
}

export function snapshot(): FocusSnapshot {
  return { otherField: inOtherField(document.activeElement), layerOpen: !!document.querySelector(LAYER) };
}

/** The first chat input of the tab that can take focus. */
function inputOf(tabId: string): HTMLTextAreaElement | null {
  const view = document.querySelector(`[data-tab-view="${CSS.escape(tabId)}"]`);
  return view?.querySelector<HTMLTextAreaElement>(`${CHAT_INPUT}:not(:disabled)`) ?? null;
}

const TRIES = 12;
const GAP_MS = 50;
let run = 0;

/** Focuses the chat input of the tab once it's drawn (the chat may still be loading). Does
 * nothing if the page was in a state where it mustn't, or the user clicked elsewhere
 * meanwhile. `from` was taken when the switch started. */
export function focusTabChat(tabId: string, from: boolean): void {
  const mine = ++run;
  if (from) return;
  let n = 0;
  const attempt = () => {
    if (mine !== run) return;
    // Also covers the user typing somewhere else while the chat was still loading.
    const s = snapshot();
    if (s.otherField) return;
    // A layer the switch closes (the quick switcher) may still be leaving; wait it out.
    const box = mayFocusChat(s) ? inputOf(tabId) : null;
    if (box) return void box.focus({ preventScroll: true });
    if (++n < TRIES) setTimeout(attempt, GAP_MS);
  };
  requestAnimationFrame(attempt);
}
