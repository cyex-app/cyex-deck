// V101-30 The browser's own right-click menu (Back, Refresh, Save as, Print) never shows.
// Parts of the app with their own menu open it themselves; elsewhere text fields get Cut,
// Copy, Paste and Select all, selected text gets Copy, links get Open and Copy link, and
// anything else gets nothing. The chat box keeps the browser's menu for its spelling fixes
// (`data-native-menu`). Dev builds show the browser's menu with Shift held, for Inspect.

import { type MenuEntry, openMenu } from '../components/menu.svelte';
import { copy } from './clipboard';

type Field = HTMLInputElement | HTMLTextAreaElement;

/** Input types that hold text someone can select and type into. */
const TEXT_TYPES = ['text', 'search', 'url', 'email', 'password', 'tel', 'number', ''];

export function guardNativeMenu(): () => void {
  const on = (e: MouseEvent) => {
    if (e.defaultPrevented) return;
    if (import.meta.env.DEV && e.shiftKey) return;
    const el = e.target instanceof Element ? e.target : null;
    if (el?.closest('[data-native-menu]')) return;
    e.preventDefault();
    const items = el ? entriesFor(el) : [];
    if (items.length) openMenu(e, items);
  };
  window.addEventListener('contextmenu', on);
  return () => window.removeEventListener('contextmenu', on);
}

function entriesFor(el: Element): MenuEntry[] {
  const field = el.closest('input, textarea');
  if (field instanceof HTMLTextAreaElement || (field instanceof HTMLInputElement && TEXT_TYPES.includes(field.type))) {
    return fieldMenu(field);
  }
  const editable = el.closest('[contenteditable=""], [contenteditable="true"], [contenteditable="plaintext-only"]');
  if (editable instanceof HTMLElement) return editableMenu(editable);
  const out: MenuEntry[] = [];
  const text = getSelection()?.toString() ?? '';
  if (text.trim()) {
    out.push({ label: 'Copy', icon: 'copy', hint: 'Ctrl+C', run: () => void copy(text, 'Text') });
  }
  const link = el.closest('a[href]');
  if (link instanceof HTMLAnchorElement && /^https?:/.test(link.href)) {
    out.push(
      'separator',
      { label: 'Open link', icon: 'popout', run: () => follow(link) },
      { label: 'Copy link', icon: 'link', run: () => void copy(link.href, 'Link') },
    );
  }
  return out;
}

/** Opens a link the way a click on it does. The page itself never follows it. */
function follow(link: HTMLAnchorElement): void {
  window.addEventListener('click', (e) => e.preventDefault(), { once: true });
  link.click();
}

/** Cut, Copy, Paste and Select all for an input or text box, keeping its undo history. */
function fieldMenu(f: Field): MenuEntry[] {
  const [start, end] = [f.selectionStart ?? 0, f.selectionEnd ?? 0];
  const picked = start !== end;
  const secret = f instanceof HTMLInputElement && f.type === 'password';
  const writable = !f.readOnly && !f.disabled;
  const again = () => {
    f.focus({ preventScroll: true });
    f.setSelectionRange?.(start, end);
  };
  return [
    { label: 'Undo', hint: 'Ctrl+Z', disabled: !writable, run: () => (f.focus(), document.execCommand('undo')) },
    'separator',
    { label: 'Cut', hint: 'Ctrl+X', disabled: !picked || secret || !writable, run: () => (again(), document.execCommand('cut')) },
    { label: 'Copy', hint: 'Ctrl+C', disabled: !picked || secret, run: () => (again(), document.execCommand('copy')) },
    { label: 'Paste', hint: 'Ctrl+V', disabled: !writable, run: () => void paste(again) },
    'separator',
    { label: 'Select all', hint: 'Ctrl+A', disabled: !f.value, run: () => (f.focus(), f.select()) },
  ];
}

function editableMenu(box: HTMLElement): MenuEntry[] {
  const sel = getSelection();
  const range = sel?.rangeCount ? sel.getRangeAt(0).cloneRange() : null;
  const picked = !!sel?.toString();
  const again = () => {
    box.focus({ preventScroll: true });
    if (range) {
      sel?.removeAllRanges();
      sel?.addRange(range);
    }
  };
  return [
    { label: 'Undo', hint: 'Ctrl+Z', run: () => (box.focus(), document.execCommand('undo')) },
    'separator',
    { label: 'Cut', hint: 'Ctrl+X', disabled: !picked, run: () => (again(), document.execCommand('cut')) },
    { label: 'Copy', hint: 'Ctrl+C', disabled: !picked, run: () => (again(), document.execCommand('copy')) },
    { label: 'Paste', hint: 'Ctrl+V', run: () => void paste(again) },
    'separator',
    { label: 'Select all', hint: 'Ctrl+A', run: () => (box.focus(), document.execCommand('selectAll')) },
  ];
}

/** Types the clipboard's text where the selection was, so Ctrl+Z takes it back. */
async function paste(restore: () => void): Promise<void> {
  const text = await navigator.clipboard.readText().catch(() => '');
  restore();
  if (text) document.execCommand('insertText', false, text);
}
