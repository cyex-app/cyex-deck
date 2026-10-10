// E2 Whether this window has focus. Moving pictures stand still while it doesn't, if the
// setting says so.

import { settings } from './settings.svelte';

class Focus {
  active = $state(typeof document === 'undefined' || document.hasFocus());

  /** Emotes, badges and paints show their still frame. */
  get still(): boolean {
    return settings.v.chat.animateOnlyFocused && !this.active;
  }

  start(): () => void {
    const on = () => (this.active = true);
    const off = () => (this.active = false);
    addEventListener('focus', on);
    addEventListener('blur', off);
    return () => {
      removeEventListener('focus', on);
      removeEventListener('blur', off);
    };
  }
}

export const focus = new Focus();
