import assert from 'node:assert/strict';
import { test } from 'node:test';

import {
  askPause,
  begin,
  brokeWhilePaused,
  canplayCheck,
  failed,
  initial,
  isJumping,
  isPaused,
  isStopped,
  isSwapping,
  jumpBegin,
  jumpEnd,
  landed,
  type LiveState,
  ownPaused,
  paused,
  played,
  playedWhilePaused,
  resumeStep,
  takePending,
  timedOut,
  toLiveStep,
  userPause,
} from './liveMachine.ts';

const f = (s: LiveState): LiveState => Object.freeze({ ...s });
const at = (phase: LiveState['phase'], more: Partial<LiveState> = {}): LiveState => f({ ...initial(), phase, ...more });
const playing = at('playing');
const swapping = at('swapping');
const jumping = at('jumping');
const pausedAt = (now = 1000, pos = 42) => f(paused(playing, now, pos));
const stopped = () => f(timedOut(pausedAt()));
const pauseEv = { videoPaused: true, live: true, holding: false };
const liveRes = { live: true, holding: false, dvr: false, hasHls: true };

test('initial state and predicates', () => {
  const s = initial();
  assert.deepEqual(s, { phase: 'idle', pendingPause: false, ownPause: false, checkStart: false, pausedAt: 0, pausedPos: 0 });
  assert.equal(isJumping(s), false);
  assert.equal(isSwapping(s), false);
  assert.equal(isPaused(s), false);
  assert.equal(isStopped(s), false);
  assert.equal(isJumping(swapping), true);
  assert.equal(isSwapping(jumping), false);
  assert.equal(isPaused(stopped()), true);
});

test('begin with and without a swap', () => {
  const p = f({ ...pausedAt(), pendingPause: true });
  const a = begin(p, true, true);
  assert.equal(a.phase, 'swapping');
  assert.equal(a.checkStart, true);
  assert.equal(a.pendingPause, false);
  assert.equal(a.pausedAt, 0);
  const b = begin(stopped(), false, false);
  assert.equal(b.phase, 'playing');
  assert.equal(b.checkStart, false);
  assert.equal(b.pausedAt, 0);
});

test('askPause only marks a jump', () => {
  assert.equal(askPause(swapping).pendingPause, true);
  assert.equal(askPause(playing), playing);
});

test('resumeStep branches', () => {
  const w = resumeStep(f({ ...swapping, pendingPause: true }), { ...liveRes, now: 0 });
  assert.equal(w.step, 'wait');
  assert.equal(w.s.pendingPause, false);
  assert.equal(w.s.phase, 'swapping');
  assert.equal(resumeStep(playing, { ...liveRes, now: 0 }).step, 'direct');
  assert.equal(resumeStep(swapping, { ...liveRes, now: 0 }).step, 'direct');
  assert.equal(resumeStep(pausedAt(), { ...liveRes, live: false, now: 9e9 }).step, 'direct');
  assert.equal(resumeStep(pausedAt(), { ...liveRes, holding: true, now: 9e9 }).step, 'direct');
  const quick = resumeStep(pausedAt(1000, 42), { ...liveRes, now: 1500 });
  assert.equal(quick.step, 'play');
  assert.equal(quick.pos, 42);
  assert.equal(quick.s.pausedAt, 0);
  assert.equal(quick.s.phase, 'playing');
  assert.equal(resumeStep(pausedAt(1000), { ...liveRes, now: 6000 }).step, 'sync');
  assert.equal(resumeStep(stopped(), { ...liveRes, now: 1500 }).step, 'reopen');
  assert.equal(resumeStep(pausedAt(1000), { ...liveRes, dvr: true, now: 6000 }).step, 'stay');
  assert.equal(resumeStep(stopped(), { ...liveRes, dvr: true, now: 6000 }).step, 'reload');
  assert.equal(resumeStep(pausedAt(1000), { ...liveRes, hasHls: false, now: 1500 }).step, 'reopen');
  const r = resumeStep(pausedAt(1000, 7), { ...liveRes, hasHls: false, dvr: true, now: 1500 });
  assert.equal(r.step, 'reload');
  assert.equal(r.pos, 7);
  assert.equal(r.s.pausedAt, 0);
});

test('V1061-01 two own pauses, one event: the user pause after it still counts', () => {
  const s = f(ownPaused(f(ownPaused(playing))));
  const a = userPause(s, pauseEv);
  assert.equal(a.mark, false);
  assert.equal(a.s.ownPause, false);
  const b = userPause(f(a.s), pauseEv);
  assert.equal(b.mark, true);
  assert.equal(played(f(ownPaused(playing))).ownPause, false);
});

test('a user pause during a swap ends it paused', () => {
  const r = userPause(swapping, pauseEv);
  assert.equal(r.mark, true);
  assert.equal(r.wasJumping, true);
  assert.equal(r.s.pendingPause, true);
  assert.equal(r.s.phase, 'playing');
  assert.equal(paused(f(r.s), 5, 1).phase, 'paused');
});

test('userPause no-ops', () => {
  for (const o of [
    { ...pauseEv, videoPaused: false },
    { ...pauseEv, live: false },
    { ...pauseEv, holding: true },
  ]) {
    const r = userPause(swapping, o);
    assert.equal(r.s, swapping);
    assert.equal(r.mark, false);
    assert.equal(r.wasJumping, false);
  }
  const r = userPause(playing, pauseEv);
  assert.equal(r.s, playing);
  assert.equal(r.mark, true);
  assert.equal(r.wasJumping, false);
});

test('timedOut stops a pause only', () => {
  assert.equal(timedOut(pausedAt()).phase, 'stopped');
  assert.equal(timedOut(playing), playing);
});

test('toLiveStep', () => {
  assert.equal(toLiveStep(jumping), null);
  assert.equal(toLiveStep(swapping), null);
  const s = toLiveStep(stopped());
  assert.ok(s);
  assert.equal(s.stopped, true);
  assert.equal(s.s.phase, 'playing');
  assert.equal(s.s.pausedAt, 0);
  const p = toLiveStep(pausedAt());
  assert.ok(p);
  assert.equal(p.stopped, false);
  assert.equal(p.s.phase, 'playing');
  assert.equal(toLiveStep(at('idle'))?.s.phase, 'idle');
});

test('jumpBegin and jumpEnd', () => {
  const j = jumpBegin(f({ ...playing, pendingPause: true }));
  assert.equal(j.phase, 'jumping');
  assert.equal(j.pendingPause, false);
  assert.equal(jumpEnd(f(j)).phase, 'playing');
  // A swap running when the jump starts stays a swap, through the jump's end.
  const sj = jumpBegin(f({ ...swapping, pendingPause: true }));
  assert.equal(sj.phase, 'swapping');
  assert.equal(sj.pendingPause, false);
  assert.equal(jumpEnd(f(sj)).phase, 'swapping');
  // A swap that started during the jump keeps its protection (V1061-01 U11).
  assert.equal(jumpEnd(swapping), swapping);
  const p = pausedAt();
  assert.equal(jumpEnd(p), p);
});

test('takePending', () => {
  const t = takePending(f({ ...playing, pendingPause: true }));
  assert.equal(t.pending, true);
  assert.equal(t.s.pendingPause, false);
  assert.equal(takePending(playing).pending, false);
});

test('landed and failed end a swap, keep a pause', () => {
  assert.equal(landed(swapping).phase, 'playing');
  assert.equal(failed(swapping).phase, 'playing');
  assert.equal(landed(jumping).phase, 'playing');
  const p = pausedAt();
  assert.equal(landed(p), p);
  assert.equal(failed(p), p);
});

test('brokeWhilePaused stops it', () => {
  const s = brokeWhilePaused(pausedAt(1000));
  assert.equal(s.phase, 'stopped');
  assert.equal(s.pausedAt, 1000);
});

test('playedWhilePaused', () => {
  const o = { live: true, hasHls: true, dvr: false };
  assert.equal(playedWhilePaused(playing, { ...o, now: 0 }).step, 'none');
  assert.equal(playedWhilePaused(pausedAt(), { ...o, live: false, now: 0 }).step, 'none');
  const k = playedWhilePaused(pausedAt(1000), { ...o, now: 1500 });
  assert.equal(k.step, 'keep');
  assert.equal(k.s.phase, 'playing');
  assert.equal(k.s.pausedAt, 0);
  assert.equal(playedWhilePaused(pausedAt(1000), { ...o, dvr: true, now: 6000 }).step, 'keep');
  const r = playedWhilePaused(stopped(), { ...o, now: 1500 });
  assert.equal(r.step, 'resume');
  assert.equal(r.s.phase, 'stopped');
  assert.equal(playedWhilePaused(pausedAt(1000), { ...o, hasHls: false, now: 1500 }).step, 'resume');
  assert.equal(playedWhilePaused(pausedAt(1000), { ...o, now: 6000 }).step, 'resume');
  assert.equal(playedWhilePaused(jumping, { ...o, now: 0 }).step, 'none');
});

test('canplayCheck consumes once', () => {
  const a = canplayCheck(f(begin(playing, false, true)));
  assert.equal(a.check, true);
  const once = f(a.s);
  const b = canplayCheck(once);
  assert.equal(b.check, false);
  assert.equal(b.s, once);
});

test('no input is mutated', () => {
  const inputs = [initial(), playing, swapping, jumping, pausedAt(), stopped(), f({ ...swapping, pendingPause: true, ownPause: true, checkStart: true })].map(f);
  const copies = inputs.map((s) => ({ ...s }));
  for (const s of inputs) {
    begin(s, true, true);
    begin(s, false, false);
    ownPaused(s);
    played(s);
    landed(s);
    askPause(s);
    resumeStep(s, { ...liveRes, now: 9e9 });
    resumeStep(s, { ...liveRes, dvr: true, now: 1 });
    toLiveStep(s);
    jumpBegin(s);
    jumpEnd(s);
    takePending(s);
    paused(s, 1, 2);
    timedOut(s);
    userPause(s, pauseEv);
    brokeWhilePaused(s);
    failed(s);
    playedWhilePaused(s, { live: true, hasHls: true, dvr: false, now: 1 });
    canplayCheck(s);
  }
  assert.deepEqual(inputs, copies);
});
