import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CLEAR_CHECKS, STALL_MS, stallStart, stallStep, type StallAction, type StallState } from './stall.ts';

/** Runs checks once a second; `pos(t)` is the play head at t ms. Returns the actions by time. */
function run(until: number, pos: (t: number) => number, o: { active?: (t: number) => boolean; ready?: (t: number) => boolean } = {}) {
  let s: StallState = stallStart(0, pos(0));
  const out: [number, StallAction][] = [];
  for (let t = 1000; t <= until; t += 1000) {
    const r = stallStep(s, { now: t, pos: pos(t), active: o.active?.(t) ?? true, ready: o.ready?.(t) ?? true });
    s = r.s;
    if (r.action !== 'none') out.push([t, r.action]);
  }
  return out;
}

test('d-4 a moving stream never stalls', () => {
  assert.deepEqual(run(60_000, (t) => t / 1000), []);
});

test('d-4 stuck 10 s: says so and reopens, then every 10 s', () => {
  assert.deepEqual(run(35_000, () => 5), [
    [STALL_MS, 'retry'],
    [2 * STALL_MS, 'retry'],
    [3 * STALL_MS, 'retry'],
  ]);
});

test('d-4 not before 10 s', () => {
  assert.deepEqual(run(9000, () => 5), []);
});

test('d-4 progress clears it', () => {
  // Stuck until 14 s, plays from there.
  const out = run(40_000, (t) => (t < 14_000 ? 5 : 5 + (t - 14_000) / 1000));
  assert.deepEqual(out, [
    [STALL_MS, 'retry'],
    [14_000 + CLEAR_CHECKS * 1000, 'clear'],
  ]);
});

test('rd-3 a reopen that plays a moment and stalls again keeps the note, retries 10 s after each try', () => {
  // Each try plays 2 s (from 3 s after it), then sticks.
  const pos = (t: number) => {
    if (t <= STALL_MS) return 5;
    const since = (t - STALL_MS) % STALL_MS;
    return 100 + Math.min(Math.max(since - 3000, 0), 2000) / 1000;
  };
  assert.deepEqual(run(35_000, pos), [
    [STALL_MS, 'retry'],
    [2 * STALL_MS, 'retry'],
    [3 * STALL_MS, 'retry'],
  ]);
});

test("rd-3 a seek's big jump isn't steady play", () => {
  const out = run(25_000, (t) => (t <= 11_000 ? 5 : 5 + (t - 11_000) * 0.01), { ready: () => true });
  // Moves 0.01 s a second: too slow to count.
  assert.deepEqual(out, [
    [STALL_MS, 'retry'],
    [2 * STALL_MS, 'retry'],
  ]);
  const jump = run(25_000, (t) => (t <= 12_000 ? 5 : t < 14_000 ? 60 : 60 + (t - 14_000) / 1000));
  assert.deepEqual(jump, [
    [STALL_MS, 'retry'],
    [14_000 + CLEAR_CHECKS * 1000, 'clear'],
  ]);
});

test("d-4 a reopen's jump isn't progress until it plays", () => {
  // The new source puts the play head at 50 s, but nothing plays (not ready).
  const out = run(25_000, (t) => (t < 11_000 ? 5 : 50), { ready: (t) => t < 11_000 });
  assert.deepEqual(out, [
    [STALL_MS, 'retry'],
    [2 * STALL_MS, 'retry'],
  ]);
});

test('d-4 paused, held or in an ad break: no stall, and it clears', () => {
  assert.deepEqual(run(30_000, () => 5, { active: () => false }), []);
  const out = run(20_000, () => 5, { active: (t) => t < 12_000 });
  assert.deepEqual(out, [
    [STALL_MS, 'retry'],
    [12_000, 'clear'],
  ]);
});
