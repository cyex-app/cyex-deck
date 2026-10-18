// rd-1 How the focused element got focus: by the keyboard or by a pointer. `:focus-visible`
// can't tell: Chromium makes a clicked element focus-visible on the first key press.

export class FocusOrigin {
  private last: 'key' | 'pointer' = 'key';
  private el: unknown = null;
  private origin: 'key' | 'pointer' = 'key';

  pointer(): void {
    this.last = 'pointer';
  }

  key(): void {
    this.last = 'key';
  }

  /** A focusin: it came from the last input (a programmatic focus after a key counts as a key's). */
  focus(el: unknown): void {
    this.el = el;
    this.origin = this.last;
  }

  /** True when `el` is focused and the keyboard put focus there. */
  byKey(el: unknown): boolean {
    return el === this.el && this.origin === 'key';
  }
}

export const focusOrigin = new FocusOrigin();

if (typeof document !== 'undefined') {
  document.addEventListener('pointerdown', () => focusOrigin.pointer(), true);
  document.addEventListener('keydown', () => focusOrigin.key(), true);
  document.addEventListener('focusin', (e) => focusOrigin.focus(e.target), true);
}

/** Whether the keyboard put focus on `el`. */
export const keyFocused = (el: unknown): boolean => focusOrigin.byKey(el);
