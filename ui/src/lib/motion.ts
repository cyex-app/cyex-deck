// Reduce motion (Windows: Settings > Accessibility > Visual effects > Animation effects).
// The CSS rule in tokens.css only covers CSS transitions and animations; script motion
// (element.animate, smooth scrolling) asks here, at the moment it moves, so a change in
// Windows applies at once.

const query = typeof matchMedia === 'function' ? matchMedia('(prefers-reduced-motion: reduce)') : null;

/** True while the user asked for less motion. */
export function reduced(): boolean {
  return query?.matches ?? false;
}

/** For scrollIntoView / scrollTo: a jump with Reduce motion on, a glide otherwise. */
export function scrollBehavior(): ScrollBehavior {
  return reduced() ? 'auto' : 'smooth';
}

// P4 #9 The motion scale of tokens.css (--fast, --base, --slow) in ms, for script motion
// (element.animate, Svelte transitions). motion.test.ts keeps them equal to the CSS.
/** Hovers, small state changes, quick exits. */
export const FAST = 120;
/** Things that appear, and short moves. */
export const BASE = 200;
/** Things that travel far: drawer height, panel moves. */
export const SLOW = 360;
/** V110-F91 The --ease curve of tokens.css, for script motion. */
export const EASE = 'cubic-bezier(0.16, 1, 0.3, 1)';

// V108-F55 What opened a menu: a key (the Menu key, Shift+F10, Enter on a button) opens it at
// once; a click lets it scale in from its trigger. Keyboard use is fast and repeated, so it
// doesn't wait on motion.
let keyed = false;
if (typeof addEventListener === 'function') {
  addEventListener('keydown', () => (keyed = true), true);
  addEventListener('pointerdown', () => (keyed = false), true);
}

/** True when the last input was a key, not the pointer. */
export function byKeyboard(): boolean {
  return keyed;
}

/** V110-A-idea Banners that appear (raid, pinned, polls and predictions): 4 px down and a hair of
 *  scale, BASE ms, like the hype train. Reduce motion: no movement, no wait. */
export function bannerIn(_node: Element, o: { reduce?: boolean } = {}): {
  duration: number;
  easing: (t: number) => number;
  css: (t: number) => string;
} {
  const none = o.reduce ?? reduced();
  return {
    duration: none ? 0 : BASE,
    easing: (t) => 1 - Math.pow(1 - t, 3),
    css: (t) => (none ? '' : `opacity:${t};transform:translateY(${(t - 1) * 4}px) scale(${0.98 + 0.02 * t})`),
  };
}

/** V110-A-idea The "More below" pill fades in, FAST ms, no movement. */
export function pillIn(_node: Element, o: { reduce?: boolean } = {}): { duration: number; css: (t: number) => string } {
  const none = o.reduce ?? reduced();
  return { duration: none ? 0 : FAST, css: (t) => (none ? '' : `opacity:${t}`) };
}
