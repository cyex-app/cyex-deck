// V105-08 Emote modifiers (C7) move on one wall clock: every copy, in every window, shows
// the same moment. Their periods (emotes.css) all divide MOD_CYCLE_MS.

export const MOD_CYCLE_MS = 6000;
/** Modifier classes that animate. */
export const MOVING = ['rainbow', 'rotate', 'shake', 'jam', 'bounce', 'slide', 'appear', 'leave'] as const;

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
