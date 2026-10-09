// Hotkeys (S2). Each action declares its default combo here; settings only store overrides.

import { settings } from './settings.svelte';

export const ACTIONS = {
  'nav.home': { label: 'Go to home', default: 'Ctrl+1' },
  'nav.discover': { label: 'Go to discover', default: 'Ctrl+2' },
  'settings.open': { label: 'Open settings', default: 'Ctrl+,' },
  'window.pin': { label: 'Pin window on top', default: 'Ctrl+Shift+P' },
  'tabs.switcher': { label: 'Go to a tab or channel', default: 'Ctrl+K' },
  'tabs.new': { label: 'Add a tab', default: 'Ctrl+T' },
  'tabs.close': { label: 'Close the tab', default: 'Ctrl+W' },
  'tabs.reopen': { label: 'Reopen closed tab', default: 'Ctrl+Shift+T' },
  'tabs.next': { label: 'Next tab', default: 'Ctrl+Tab' },
  'tabs.prev': { label: 'Previous tab', default: 'Ctrl+Shift+Tab' },
} as const satisfies Record<string, { label: string; default: string }>;

export type ActionId = keyof typeof ACTIONS;

export const ACTION_IDS = Object.keys(ACTIONS) as ActionId[];

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
  return settings.v.hotkeys?.bindings?.[id] ?? ACTIONS[id].default;
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

/** Runs the handlers for their bound combos while the calling component is mounted. */
export function useHotkeys(handlers: Partial<Record<ActionId, () => void>>): void {
  $effect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (recording.active) return;
      const combo = comboFromEvent(e);
      if (!combo) return;
      // Plain keys belong to text fields; only Ctrl/Alt combos fire while typing.
      if (isTyping(e.target) && !/^(Ctrl|Alt)\+/.test(combo)) return;
      const id = actionFor(combo);
      const run = id && handlers[id];
      if (run) {
        e.preventDefault();
        run();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });
}
