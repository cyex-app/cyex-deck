// E2 Whether this window has focus. Moving pictures stand still while it doesn't, if the
// setting says so.

import { settings } from './settings.svelte';

class Focus {
  active = $state(typeof document === 'undefined' || document.hasFocus());

  visible = $state(typeof document === 'undefined' || document.visibilityState === 'visible');

  /** Emotes, badges and paints show their still frame. */
  get still(): boolean {
    return settings.v.chat.animateOnlyFocused && !this.active;
  }

  /** V1051-03 The cosmetics menu stands still whenever the window isn't in front, setting or not. */
  get idle(): boolean {
    return !this.active || !this.visible;
  }

  start(): () => void {
    const on = () => (this.active = true);
    const off = () => (this.active = false);
    // V110-F04 A missed focus event could keep emotes still for good: a click or the window
    // showing again reads the real state.
    const check = () => (this.active = document.hasFocus());
    const vis = () => {
      this.visible = document.visibilityState === 'visible';
      check();
    };
    addEventListener('focus', on);
    document.addEventListener('visibilitychange', vis);
    addEventListener('blur', off);
    addEventListener('pointerdown', check, { capture: true, passive: true });
    return () => {
      removeEventListener('focus', on);
      removeEventListener('blur', off);
      removeEventListener('pointerdown', check, { capture: true });
      document.removeEventListener('visibilitychange', vis);
    };
  }
}

export const focus = new Focus();
