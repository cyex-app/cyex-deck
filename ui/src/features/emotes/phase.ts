// V105-08 Emote modifiers (C7) move on one wall clock: every copy, in every window, shows
// the same moment. Their periods all divide MOD_CYCLE_MS.

export const MOD_CYCLE_MS = 6000;

/** V111-20 Each moving modifier's animation, timed as BTTV and FFZ time them. The keyframes are
 *  in emotes.css; they move only translate, rotate, scale and filter, so the compositor runs them
 *  and a flip (transform) stays under a spin. */
export const MOTION = {
  rainbow: 'emote-rainbow 2s linear infinite',
  party: 'emote-party 1.5s linear infinite',
  rotate: 'emote-spin 1.5s linear infinite',
  shake: 'emote-shake 0.1s linear infinite',
  jitter: 'emote-jitter 0.5s step-start infinite',
  jam: 'emote-jam 0.6s linear infinite',
  bounce: 'emote-bounce 0.5s linear infinite',
  slide: 'emote-slide 1s linear infinite',
  appear: 'emote-appear 3s linear infinite',
  leave: 'emote-leave 3s linear infinite',
} as const;

/** Modifier classes that animate. */
export const MOVING = Object.keys(MOTION) as (keyof typeof MOTION)[];

/** FFZ: appear and leave together come in and go out over 6 s. */
export const IN_OUT = 'emote-in-out 6s linear infinite';

/** How long one slide across takes, all dividing the cycle. */
export const SLIDE_SECS = [0.5, 0.75, 1, 1.2, 1.5, 2, 3] as const;

/** FFZ slides at about two heights a second: half a second per square, longer for wide emotes. */
export function slideSecs(ratio: number): number {
  const want = 0.5 * Math.max(1, ratio);
  return SLIDE_SECS.find((s) => s >= want - 1e-9) ?? SLIDE_SECS[SLIDE_SECS.length - 1];
}

/** The CSS `animation` list for these moving modifier names, '' when none moves. */
export function motion(names: readonly string[], slide = 1): string {
  const on = new Set(names);
  const out: string[] = [];
  if (on.has('appear') && on.has('leave')) {
    out.push(IN_OUT);
    on.delete('appear');
    on.delete('leave');
  }
  // FFZ doesn't spin what slides.
  if (on.has('slide')) on.delete('rotate');
  for (const [name, a] of Object.entries(MOTION)) {
    if (on.has(name)) out.push(name === 'slide' ? a.replace(' 1s ', ` ${slide}s `) : a);
  }
  return out.join(', ');
}

/** Start time on a document's timeline that puts a modifier animation in step with the wall clock. */
export function wallStart(timeOrigin: number): number {
  return -(timeOrigin % MOD_CYCLE_MS);
}

export function moves(classes: string): boolean {
  const list = classes.split(' ');
  return MOVING.some((m) => list.includes(`m-${m}`));
}

/** Puts the running modifier animations of `target` (an element, or the whole document) in step. */
export function align(target: Element | Document): void {
  const start = wallStart(performance.timeOrigin);
  for (const a of target.getAnimations()) {
    if (a instanceof CSSAnimation && a.animationName.startsWith('emote-') && a.playState === 'running') a.startTime = start;
  }
}

/** Svelte action for an emote picture: in step from its first frame when a modifier moves it. */
export function inStep(node: Element, on: boolean): void {
  if (on) align(node);
}
