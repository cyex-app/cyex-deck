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
