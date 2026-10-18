import assert from 'node:assert/strict';
import { test } from 'node:test';

import { CLEAR_CHECKS, CLEAR_NEW_CHECKS, STALL_MS, stallStart, stallStep, type StallAction, type StallState } from './stall.ts';

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

/** qd-3 Like `run`, with program date times: `date(t)` at the play head, `edge(t)` the playlist's newest. */
function runDated(until: number, pos: (t: number) => number, date: (t: number) => number | null, edge: (t: number) => number | null) {
  let s: StallState = stallStart(0, pos(0));
  const out: [number, StallAction][] = [];
  for (let t = 1000; t <= until; t += 1000) {
    const r = stallStep(s, { now: t, pos: pos(t), active: true, ready: true, date: date(t), edge: edge(t) });
    s = r.s;
    if (r.action !== 'none') out.push([t, r.action]);
  }
  return out;
}

test('qd-3 a reopen that replays the last segments of a stuck stream keeps the note', () => {
  const W = 100_000;
  // Plays till 6 s (content up to W + 6 s), then sticks. From 17 s each try replays 4 s of old
  // content (W + 2..6 s) on a new play head, then sticks again.
  const replay = (t: number) => (t < 17_000 ? null : Math.min(((t - 17_000) % STALL_MS) / 1000, 4));
  const pos = (t: number) => (replay(t) === null ? Math.min(t, 6000) / 1000 : 50 + replay(t)!);
  const date = (t: number) => (replay(t) === null ? W + Math.min(t, 6000) : W + 2000 + replay(t)! * 1000);
  assert.deepEqual(runDated(45_000, pos, date, () => W + 6000), [
    [16_000, 'retry'],
    [26_000, 'retry'],
    [36_000, 'retry'],
  ]);
});

test('qd-3 the stuck stream goes on: it tries at once and clears on new content', () => {
  const W = 100_000;
  // Stuck at W + 6 s from 6 s on. At 30 s the playlist grows; from 32 s new content plays.
  const pos = (t: number) => (t < 32_000 ? Math.min(t, 6000) / 1000 : 200 + (t - 32_000) / 1000);
  const date = (t: number) => (t < 32_000 ? W + Math.min(t, 6000) : W + 40_000 + (t - 32_000));
  const edge = (t: number) => (t < 30_000 ? W + 6000 : W + t);
  // New content counts from the jump to it (32 s) on.
  assert.deepEqual(runDated(40_000, pos, date, edge), [
    [16_000, 'retry'],
    [26_000, 'retry'],
    [30_000, 'retry'],
    [32_000 + (CLEAR_NEW_CHECKS - 1) * 1000, 'clear'],
  ]);
});

test('qd-3 a playlist that grows while nothing plays waits the 10 s', () => {
  const W = 100_000;
  const out = runDated(30_000, (t) => Math.min(t, 6000) / 1000, (t) => W + Math.min(t, 6000), (t) => W + 8000 + t);
  assert.deepEqual(out, [
    [16_000, 'retry'],
    [26_000, 'retry'],
  ]);
});
