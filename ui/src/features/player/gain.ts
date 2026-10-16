// V105-10 The boost's gain, put between a player's video and the speakers the first time
// it's boosted. A media element can't leave its audio graph again, so later it just
// stays at gain 1. Changes glide, so toggling never clicks.
// A limiter after the gain keeps loud parts from clipping (LIMITER in boost.ts).
// V1051-01 Its own AudioContext (the app's chime plays in Rust now), on the streams' device.
import { LIMITER, LIMITER_MAKEUP } from './boost';
import { routeContext } from './output.svelte';

const gains = new WeakMap<HTMLMediaElement, { node: GainNode; chain: AudioNode[] }>();
/** Time constant of a gain change, in seconds. */
const GLIDE = 0.02;

let boostCtx: AudioContext | undefined;

/** The window's one boost context, made the first time a player is boosted. */
function boostContext(): AudioContext {
  if (!boostCtx) {
    boostCtx = new AudioContext();
    routeContext(boostCtx);
  }
  return boostCtx;
}

/** Sets the boost's gain on `video`. Builds the graph only when `build` (the player is boosted). */
export function setGain(video: HTMLMediaElement, value: number, build: boolean): void {
  let node = gains.get(video)?.node;
  // Players never boosted don't get an AudioContext at all.
  if (!node && !build) return;
  const ctx = boostContext();
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
    const source = ctx.createMediaElementSource(video);
    source.connect(node).connect(limiter).connect(makeup).connect(ctx.destination);
    gains.set(video, { node, chain: [source, node, limiter, makeup] });
  }
  if (ctx.state === 'suspended') void ctx.resume();
  node.gain.setTargetAtTime(value, ctx.currentTime, GLIDE);
}

/** Takes a closed player's boost graph out of the context, so channel switches don't pile them up. */
export function releaseGain(video: HTMLMediaElement): void {
  const g = gains.get(video);
  if (!g) return;
  gains.delete(video);
  for (const n of g.chain) n.disconnect();
}
