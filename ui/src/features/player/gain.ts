// V105-10 The boost's gain, put between a player's video and the speakers the first time
// it's boosted. A media element can't leave its audio graph again, so later it just
// stays at gain 1. Changes glide, so toggling never clicks.
// A limiter after the gain keeps loud parts from clipping (LIMITER in boost.ts).
import { audio } from '../../lib/sound';
import { LIMITER, LIMITER_MAKEUP } from './boost';

const gains = new WeakMap<HTMLMediaElement, GainNode>();
/** Time constant of a gain change, in seconds. */
const GLIDE = 0.02;

/** Sets the boost's gain on `video`. Builds the graph only when `build` (the player is boosted). */
export function setGain(video: HTMLMediaElement, value: number, build: boolean): void {
  let node = gains.get(video);
  // Players never boosted don't get an AudioContext at all.
  if (!node && !build) return;
  const ctx = audio();
  if (!node) {
    node = ctx.createGain();
    node.gain.value = value;
    const limiter = ctx.createDynamicsCompressor();
    limiter.threshold.value = LIMITER.threshold;
    limiter.knee.value = LIMITER.knee;
    limiter.ratio.value = LIMITER.ratio;
    limiter.attack.value = LIMITER.attack;
    limiter.release.value = LIMITER.release;
    const makeup = ctx.createGain();
    makeup.gain.value = LIMITER_MAKEUP;
    ctx.createMediaElementSource(video).connect(node).connect(limiter).connect(makeup).connect(ctx.destination);
    gains.set(video, node);
  }
  if (ctx.state === 'suspended') void ctx.resume();
  node.gain.setTargetAtTime(value, ctx.currentTime, GLIDE);
}
