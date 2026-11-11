// V110-B7 The hidden title bar of theatre: shown while the pointer is in the top 4 px or on
// the bar, or focus is in it; hidden 600 ms after. One instance per window.

import { HIDDEN, hideIn, type Reveal, revealStep } from './theatreBar';

const BAR = '.chrome, .controls';

export class TheatreBar {
  shown = $state(false);
  private reveal: Reveal = HIDDEN;
  private y: number | null = null;
  private over = false;
  private timer = 0;

  private step(): void {
    clearTimeout(this.timer);
    const now = performance.now();
    const a = document.activeElement;
    // Keyboard focus only: a clicked button keeps focus without asking the bar to stay.
    const focusIn = !!a?.closest(BAR) && a.matches(':focus-visible');
    this.reveal = revealStep(this.reveal, { y: this.y, overBar: this.over, focusIn, now });
    this.shown = this.reveal.shown;
    const wait = hideIn(this.reveal, now);
    if (wait !== null) this.timer = window.setTimeout(() => this.step(), wait);
  }

  /** Starts listening; the returned function stops and hides the bar. */
  start(): () => void {
    const move = (e: PointerEvent) => {
      this.y = e.clientY;
      this.over = !!(e.target as Element | null)?.closest?.(BAR);
      this.step();
    };
    const leave = () => {
      this.y = null;
      this.over = false;
      this.step();
    };
    const focus = () => this.step();
    window.addEventListener('pointermove', move, true);
    document.documentElement.addEventListener('pointerleave', leave);
    document.addEventListener('focusin', focus);
    document.addEventListener('focusout', focus);
    return () => {
      window.removeEventListener('pointermove', move, true);
      document.documentElement.removeEventListener('pointerleave', leave);
      document.removeEventListener('focusin', focus);
      document.removeEventListener('focusout', focus);
      clearTimeout(this.timer);
      this.reveal = HIDDEN;
      this.y = null;
      this.over = false;
      this.shown = false;
    };
  }
}
