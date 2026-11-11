// Hotkeys (S2). Each action declares its default combo here; settings only store overrides.
// `global` ones work while another app is in front; Rust registers those (app/src/hotkeys.rs).

import { keyTaken, layers } from './escape';
import { settings } from './settings.svelte';
import { zoomHotkeys } from './zoom';

import { ACTIONS, comboAlias, OVER_SHEET, repeatAllowed, resolveBinding, typesChar } from './hotkeyActions';
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

/** What a key press runs: its own binding, else the one its V107-N6a alias has. */
function actionOfPress(combo: string): ActionId | undefined {
  const alias = comboAlias(combo);
  return actionFor(combo) ?? (alias ? actionFor(alias) : undefined);
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
  // A slider takes only arrows: Space, M and the rest still reach the hotkeys.
  if (target instanceof HTMLInputElement && target.type === 'range') return false;
  return target instanceof HTMLElement && (target.isContentEditable || /^(INPUT|TEXTAREA)$/.test(target.tagName));
}

/** V108-S1 The settings sheet is open: only the OVER_SHEET keys reach the tabs and players behind it. */
export const keyHold = $state({ sheet: false });

/** V110-N4 Reload the app (Ctrl+F5), also over the settings sheet. */
export const appHotkeys = { 'app.reload': () => location.reload() };

/** Runs the handlers for their bound combos while the calling component is mounted. A
 *  handler that returns `false` leaves the key to the page, e.g. Space on a button. */
export function useHotkeys(own: Partial<Record<ActionId, (e: KeyboardEvent) => unknown>>): void {
  const handlers = { ...zoomHotkeys, ...appHotkeys, ...own };
  $effect(() => {
    let consumed: ActionId | undefined;
    const onKey = (e: KeyboardEvent) => {
      // An open dialog, menu or select has the keyboard; so does a key something already used.
      if (recording.active) return;
      // V110-F81 AltGr (@, € on German or Polish layouts) is typing, not Ctrl+Alt.
      if (typesChar(e)) return;
      const combo = comboFromEvent(e);
      if (!combo) return;
      // An open menu doesn't hold back the switcher: it opens and the menu goes (030e896).
      const switcherOverMenu = actionOfPress(combo) === 'tabs.switcher' && !e.defaultPrevented && layers.onlyMenus();
      if (keyTaken(e) && !switcherOverMenu) return;
      const held = actionOfPress(combo);
      if (keyHold.sheet && !(held && OVER_SHEET.has(held))) return;
      // Plain keys belong to text fields; only Ctrl/Alt combos fire while typing.
      if (isTyping(e.target) && !/^(Ctrl|Alt)\+/.test(combo)) return;
      const id = actionOfPress(combo);
      // V110-F81 A held toggle (play, mute, fullscreen) runs once; its repeats are still kept from the page.
      if (id && !repeatAllowed(id, e.repeat)) {
        if (consumed === id) e.preventDefault();
        return;
      }
      const run = id && handlers[id];
      const used = !!run && run(e) !== false;
      if (used) e.preventDefault();
      consumed = used ? id : undefined;
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
}
