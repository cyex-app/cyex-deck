// V1061-01 U11 The live player's pause, jump and swap state, pure. engine.svelte.ts does the media calls.

import { resumePlan } from './liveEdge.ts';

export type Phase = 'idle' | 'playing' | 'paused' | 'stopped' | 'jumping' | 'swapping';

export interface LiveState {
  /** idle: before the first start, acts as playing. */
  readonly phase: Phase;
  /** The user paused during our silent jump or swap: it ends paused. */
  readonly pendingPause: boolean;
  /** Our own pause whose event hasn't come yet. A flag, not a count: a swap's source reload
   *  drops the queued pause event, and a stale count ate the user's next pause (V1061-01). */
  readonly ownPause: boolean;
  /** A rewind start checks once that it landed where it was asked to. */
  readonly checkStart: boolean;
  /** When it was paused (performance.now()); non-zero exactly when phase is paused or stopped. */
  readonly pausedAt: number;
  readonly pausedPos: number;
}

/** direct: not a live pause, it just plays. The rest is resumePlan's. */
export type ResumeStep = 'wait' | 'direct' | 'play' | 'sync' | 'reopen' | 'stay' | 'reload';

export const isJumping = (s: LiveState): boolean => s.phase === 'jumping' || s.phase === 'swapping';
export const isSwapping = (s: LiveState): boolean => s.phase === 'swapping';
export const isPaused = (s: LiveState): boolean => s.phase === 'paused' || s.phase === 'stopped';
/** Q16 a Paused 10 min, or the session went away: nothing loads anymore. */
export const isStopped = (s: LiveState): boolean => s.phase === 'stopped';

export function initial(): LiveState {
  return { phase: 'idle', pendingPause: false, ownPause: false, checkStart: false, pausedAt: 0, pausedPos: 0 };
}

/** A start; `swap` when a source plays already (Q14 a, silent). rd-2 A reopen while paused
 *  lands paused: the new source is paused again (and its buffering) once its levels are in. */
export function begin(s: LiveState, swap: boolean, dvr: boolean): LiveState {
  return { ...s, phase: swap ? 'swapping' : 'playing', pendingPause: isPaused(s), checkStart: dvr, pausedAt: 0 };
}

export const ownPaused = (s: LiveState): LiveState => ({ ...s, ownPause: true });

/** Any own pause event came first or was dropped. */
export const played = (s: LiveState): LiveState => ({ ...s, ownPause: false });

/** The first frame plays: a jump or swap is over. */
export const landed = (s: LiveState): LiveState => (isJumping(s) ? { ...s, phase: 'playing' } : s);

export const askPause = (s: LiveState): LiveState => (isJumping(s) ? { ...s, pendingPause: true } : s);

export function resumeStep(
  s: LiveState,
  o: { live: boolean; holding: boolean; dvr: boolean; hasHls: boolean; now: number },
): { s: LiveState; step: ResumeStep; pos: number } {
  const pos = s.pausedPos;
  // Played again mid-jump after a pause: the jump plays when it lands.
  if (isJumping(s) && s.pendingPause) return { s: { ...s, pendingPause: false }, step: 'wait', pos };
  if (!o.live || o.holding || !isPaused(s)) return { s, step: 'direct', pos };
  const step = resumePlan({ pausedMs: o.now - s.pausedAt, inVod: o.dvr, stopped: isStopped(s) || !o.hasHls });
  return { s: { ...s, phase: 'playing', pausedAt: 0 }, step, pos };
}

/** null while a jump runs; `stopped`: the session must reopen. */
export function toLiveStep(s: LiveState): { s: LiveState; stopped: boolean } | null {
  if (isJumping(s)) return null;
  return { s: { ...s, pausedAt: 0, phase: isPaused(s) ? 'playing' : s.phase }, stopped: isStopped(s) };
}

/** A swap already running keeps its phase (V1061-01 U11). */
export const jumpBegin = (s: LiveState): LiveState => ({ ...s, phase: isSwapping(s) ? 'swapping' : 'jumping', pendingPause: false });

/** A swap that started during the jump keeps its own phase. */
export const jumpEnd = (s: LiveState): LiveState => (s.phase === 'jumping' ? { ...s, phase: 'playing' } : s);

export const takePending = (s: LiveState): { s: LiveState; pending: boolean } => ({
  s: { ...s, pendingPause: false },
  pending: s.pendingPause,
});

export const paused = (s: LiveState, now: number, pos: number): LiveState => ({ ...s, phase: 'paused', pausedAt: now, pausedPos: pos });

export const timedOut = (s: LiveState): LiveState => (s.phase === 'paused' ? { ...s, phase: 'stopped' } : s);

/** A pause event. `mark`: it's the user's. `wasJumping`: it ended a jump or swap. */
export function userPause(
  s: LiveState,
  o: { videoPaused: boolean; live: boolean; holding: boolean },
): { s: LiveState; mark: boolean; wasJumping: boolean } {
  if (s.ownPause) return { s: { ...s, ownPause: false }, mark: false, wasJumping: false };
  // Played again before the event came: nothing to keep.
  if (!o.videoPaused || !o.live || o.holding) return { s, mark: false, wasJumping: false };
  const wasJumping = isJumping(s);
  return { s: wasJumping ? { ...s, pendingPause: true, phase: 'playing' } : s, mark: true, wasJumping };
}

/** V1061-01 The session went away while paused: play reopens quietly. */
export const brokeWhilePaused = (s: LiveState): LiveState => ({ ...s, phase: 'stopped' });

export const failed = (s: LiveState): LiveState => (isJumping(s) ? { ...s, phase: 'playing' } : s);

/** A play that skipped `resume()`. keep: it already plays where it should; resume: send it there. */
export function playedWhilePaused(
  s: LiveState,
  o: { live: boolean; hasHls: boolean; dvr: boolean; now: number },
): { s: LiveState; step: 'none' | 'keep' | 'resume' } {
  if (!o.live || isJumping(s) || !isPaused(s)) return { s, step: 'none' };
  if (o.hasHls && !isStopped(s) && (o.dvr || o.now - s.pausedAt < 1000)) return { s: { ...s, phase: 'playing', pausedAt: 0 }, step: 'keep' };
  return { s, step: 'resume' };
}

export function canplayCheck(s: LiveState): { s: LiveState; check: boolean } {
  return s.checkStart ? { s: { ...s, checkStart: false }, check: true } : { s, check: false };
}
