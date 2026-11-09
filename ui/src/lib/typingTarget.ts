// V110-F83 Is a key event's target a place where someone types (so arrows and letters are theirs)?
// Same idea as `isTyping` in hotkeys.svelte.ts, plus select, and written on plain fields so it tests without a DOM.

interface Target {
  readonly tagName?: string;
  readonly isContentEditable?: boolean;
  readonly type?: string;
}

export function typingTarget(t: EventTarget | Target | null): boolean {
  const el = t as Target | null;
  if (!el || typeof el.tagName !== 'string') return false;
  if (el.isContentEditable) return true;
  // A slider takes only arrows, but a story is not its business either way.
  return /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName);
}
