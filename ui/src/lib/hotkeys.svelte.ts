// Hotkeys (S2). Each action declares its default combo here; settings only store overrides.
// `global` ones work while another app is in front; Rust registers those (app/src/hotkeys.rs).

import { keyTaken, layers } from './escape';
import { settings } from './settings.svelte';
import { zoomHotkeys } from './zoom';

import { ACTIONS, resolveBinding } from './hotkeyActions';
export { ACTIONS };

export type ActionId = keyof typeof ACTIONS;

export const ACTION_IDS = Object.keys(ACTIONS) as ActionId[];

export function isGlobal(id: ActionId): boolean {
  return 'global' in ACTIONS[id];
}

/** True while the hotkey editor is listening, so shortcuts don't fire. */
export const recording = $state({ active: false });

const MODIFIER_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta']);

/** `Ctrl+Shift+K` style name for a key press, or null for a lone modifier. */
export function comboFromEvent(e: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');
  if (e.metaKey) parts.push('Win');
  parts.push(keyName(e));
  return parts.join('+');
}

function keyName(e: KeyboardEvent): string {
  // Physical letters and digits, so Shift+1 stays "Shift+1" instead of "!".
  if (e.code.startsWith('Key')) return e.code.slice(3);
  if (e.code.startsWith('Digit')) return e.code.slice(5);
  if (e.key === ' ') return 'Space';
  return e.key.length === 1 ? e.key.toUpperCase() : e.key;
}

/** Current combo for an action. Empty string = unbound. */
export function bindingOf(id: ActionId): string {
  return resolveBinding(id, settings.v.hotkeys?.bindings);
}

export function actionFor(combo: string): ActionId | undefined {
  return ACTION_IDS.find((id) => bindingOf(id) === combo);
}

/** Binds `combo` to `id`. Returns the action that lost the combo, if any. */
export async function rebind(id: ActionId, combo: string): Promise<ActionId | undefined> {
  const previous = combo ? actionFor(combo) : undefined;
  const bindings: Record<string, string> = { [id]: combo };
  if (previous && previous !== id) bindings[previous] = '';
  await settings.patch('hotkeys', { bindings });
  return previous !== id ? previous : undefined;
}

export function resetBinding(id: ActionId): Promise<void> {
  return settings.patch('hotkeys', { bindings: { [id]: null } });
}

function isTyping(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA)$/.test(target.tagName));
}

/** Runs the handlers for their bound combos while the calling component is mounted. A
 *  handler that returns `false` leaves the key to the page, e.g. Space on a button. */
export function useHotkeys(own: Partial<Record<ActionId, (e: KeyboardEvent) => unknown>>): void {
  const handlers = { ...zoomHotkeys, ...own };
  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      // An open dialog, menu or select has the keyboard; so does a key something already used.
      if (recording.active) return;
      const combo = comboFromEvent(e);
      if (!combo) return;
      // An open menu doesn't hold back the switcher: it opens and the menu goes (030e896).
      const switcherOverMenu = actionFor(combo) === 'tabs.switcher' && !e.defaultPrevented && layers.onlyMenus();
      if (keyTaken(e) && !switcherOverMenu) return;
      // Plain keys belong to text fields; only Ctrl/Alt combos fire while typing.
      if (isTyping(e.target) && !/^(Ctrl|Alt)\+/.test(combo)) return;
      const id = actionFor(combo);
      const run = id && handlers[id];
      if (run && run(e) !== false) e.preventDefault();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
}
