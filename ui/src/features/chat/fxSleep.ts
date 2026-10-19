// V107-H1 Channel-point line effects (1.42) rest when nobody can see them: the line
// scrolled off screen or the window hidden. One observer for all lines. Resting, the
// effect gets `asleep` (its spin pauses) and its video pauses; a removed line's video is emptied.

import { onShown, pageHidden } from '../../lib/awake';

const seen = new Map<Element, (onScreen: boolean) => void>();
let io: IntersectionObserver | null = null;

function observer(): IntersectionObserver | null {
  if (io || typeof IntersectionObserver !== 'function') return io;
  io = new IntersectionObserver((entries) => {
    for (const e of entries) seen.get(e.target)?.(e.isIntersecting);
  });
  return io;
}

/** Svelte action for the `.fx` layer of a line. */
export function fxSleep(el: HTMLElement): { destroy(): void } {
  let onScreen = true;
  let hidden = pageHidden();
  let awake = true;
  const apply = () => {
    // Moves only on screen in a shown window.
    const now = onScreen && !hidden;
    if (now === awake) return;
    awake = now;
    el.classList.toggle('asleep', !now);
    const v = el.querySelector('video');
    if (!v) return;
    if (now) void v.play().catch(() => {});
    else v.pause();
  };
  seen.set(el, (on) => {
    onScreen = on;
    apply();
  });
  observer()?.observe(el);
  const unShown = onShown((h) => {
    hidden = h;
    apply();
  });
  apply();
  return {
    destroy() {
      unShown();
      seen.delete(el);
      io?.unobserve(el);
      // L2 A dropped line lets go of its decoder now, not whenever the element is collected.
      const v = el.querySelector('video');
      if (v) {
        v.pause();
        v.removeAttribute('src');
        v.load();
      }
    },
  };
}
