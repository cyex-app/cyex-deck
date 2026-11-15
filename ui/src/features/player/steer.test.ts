import assert from 'node:assert/strict';
import { test } from 'node:test';

import { edgeJump } from './liveEdge.ts';
import {
  AWAY_SPARE,
  BUMP_GAP,
  CALM,
  MARGIN_MAX,
  MARGIN_STEP,
  NEAR_DRY,
  RATE_FAST,
  RATE_SLOW,
  REST,
  SPARE_KEEP,
  Steer,
} from './steer.ts';

/** Feeds a buffer that swings like whole 2 s segments: `low` right before a reload, `low + 2` after. */
function feed(s: Steer, from: number, to: number, low: number): number {
  let r = 1;
  for (let t = from; t < to; t += 0.25) r = s.step(t, low + 2 - ((t % 2) / 1.75) * 2);
  return r;
}

test('V108-LL2 spare buffer speeds up, only after a full window', () => {
  const s = new Steer();
  assert.equal(feed(s, 0, 3, 1.7), 1);
  assert.equal(feed(s, 3, 6, 1.7), RATE_FAST);
});

test('V108-LL2 fast settles back to 1x once the spare is used up', () => {
  const s = new Steer();
  feed(s, 0, 6, 1.7);
  assert.equal(feed(s, 6, 8, 1.0), RATE_FAST, 'still spare: keeps going');
  assert.equal(feed(s, 8, 14, 0.3), 1);
});

test('V108-LL2 a thin buffer slows down until it is safe again', () => {
  const s = new Steer();
  assert.equal(feed(s, 0, 6, 0.2), RATE_SLOW);
  assert.equal(feed(s, 6, 12, 0.3), RATE_SLOW, 'between low and safe: stays slow');
  assert.equal(feed(s, 12, 18, 0.7), 1);
});

test('V108-LL2 a steady buffer in the band stays at 1x', () => {
  const s = new Steer();
  assert.equal(feed(s, 0, 20, 0.6), 1);
});

test('V108-LL2 reset goes back to 1x and waits for a new window', () => {
  const s = new Steer();
  feed(s, 0, 6, 1.7);
  s.reset();
  assert.equal(s.rate, 1);
  assert.equal(feed(s, 6, 8, 1.7), 1);
});

test('V108-LL6 after a dry run it keeps more spare: no speed up at a spare that used to', () => {
  const s = new Steer();
  s.dry(0);
  assert.equal(s.margin, MARGIN_STEP);
  // 1.1 s spare: fast before (over 0.9), now under 0.9 + 0.25 and over 0.25 + 0.25.
  assert.equal(feed(s, 0, 10, 1.1), 1);
  // 0.45 s spare is thin now: it slows down to build the margin back.
  assert.equal(feed(s, 10, 20, 0.45), RATE_SLOW);
});

test('V108-LL6 the margin adds up to a cap and survives a reset', () => {
  const s = new Steer();
  for (let i = 0; i < 10; i++) s.dry(i * BUMP_GAP);
  assert.equal(s.margin, MARGIN_MAX);
  s.reset();
  assert.equal(s.margin, MARGIN_MAX, 'a pause or a jump keeps it');
});

test('V108-LL6 back at 1x it rests before speeding up again', () => {
  const s = new Steer();
  feed(s, 0, 6, 1.7);
  assert.equal(s.rate, RATE_FAST);
  // Spare used up: back to 1x at t≈8.
  let t = 6;
  for (; s.rate !== 1; t += 0.25) s.step(t, 0.45);
  const back = t;
  // Plenty again at once: it waits REST s at 1x first.
  for (; t < back + REST - 0.5; t += 0.25) assert.equal(s.step(t, 2), 1);
  for (; t < back + REST + 4.5; t += 0.25) s.step(t, 2);
  assert.equal(s.rate, RATE_FAST);
});

test('V108-LL6 a thin buffer still slows down at once, rest or not', () => {
  const s = new Steer();
  feed(s, 0, 6, 1.7);
  let t = 6;
  for (; s.rate !== 1; t += 0.25) s.step(t, 0.45);
  for (const end = t + 0.5; t < end; t += 0.25) s.step(t, 0.1);
  assert.equal(s.rate, RATE_SLOW);
});

test('V108-P10 out of sight it slows down to keep AWAY_SPARE more, back in sight it speeds up again', () => {
  const s = new Steer();
  // 1.7 s least buffer: spare enough in sight.
  assert.equal(feed(s, 0, 6, 1.7), RATE_FAST);
  s.away = true;
  assert.equal(feed(s, 6, 12, 1.7), RATE_SLOW, 'out of sight 1.7 s is thin');
  assert.equal(feed(s, 12, 18, 0.5 + AWAY_SPARE), 1, 'the extra spare is there: 1x, no speed up');
  s.away = false;
  assert.equal(feed(s, 18, 30, 0.5 + AWAY_SPARE), RATE_FAST, 'in sight again: the extra goes');
});

test('V108-P10 the away spare is not a dry-run margin: it neither wears off nor stays after', () => {
  const s = new Steer();
  s.away = true;
  feed(s, 0, 6, 0.5);
  assert.equal(s.margin, 0);
  s.away = false;
  assert.equal(feed(s, 6, 12, 0.5), 1);
});

/** Feeds a steady `ahead` (progressive loads: the buffer hardly swings), every 0.25 s. */
function steady(s: Steer, from: number, to: number, ahead: number): number {
  let r = 1;
  for (let t = from; t < to; t += 0.25) r = s.step(t, ahead);
  return r;
}

test('V111-38 a near-dry reading grows the margin, once per trough', () => {
  const s = new Steer();
  steady(s, 0, 6, 0.6);
  assert.equal(s.margin, 0, 'a steady stream earns nothing');
  s.step(6, NEAR_DRY / 2);
  assert.equal(s.margin, MARGIN_STEP);
  steady(s, 6.25, 6 + BUMP_GAP - 0.25, NEAR_DRY / 2);
  assert.equal(s.margin, MARGIN_STEP, 'the same trough counts once');
  s.step(6 + BUMP_GAP, NEAR_DRY / 2);
  assert.equal(s.margin, 2 * MARGIN_STEP, 'still dry a gap later: one more');
});

test('V111-38 a near-dry reading and the stall it ends in count once', () => {
  const s = new Steer();
  steady(s, 0, 6, 0.6);
  s.step(6, 0.05);
  s.dry(6.5);
  assert.equal(s.margin, MARGIN_STEP);
});

test('V111-38 near-dry readings before a full window or out of sight add nothing', () => {
  const s = new Steer();
  s.step(0, 0.02);
  s.step(1, 0.02);
  assert.equal(s.margin, 0, 'a fresh start or seek is low for a moment');
  s.away = true;
  steady(s, 1.25, 20, 0.02);
  assert.equal(s.margin, 0, "a hidden page's timers are late, not the stream");
});

test('V111-38 a stream that keeps running thin earns up to MARGIN_MAX, no more', () => {
  const s = new Steer();
  steady(s, 0, 6, 0.6);
  for (let t = 6; t < 120; t += BUMP_GAP + 1) {
    steady(s, t, t + BUMP_GAP + 1, 0.6);
    s.step(t + BUMP_GAP + 1, 0.05);
  }
  assert.equal(s.margin, MARGIN_MAX);
});

test('V111-38 calm play takes a step off every CALM s, down to 0', () => {
  const s = new Steer();
  for (let i = 0; i < 4; i++) s.dry(i * BUMP_GAP);
  assert.equal(s.margin, MARGIN_MAX);
  steady(s, 20, 20 + CALM - 1, 1.5);
  assert.equal(s.margin, MARGIN_MAX, 'not yet');
  steady(s, 20 + CALM - 1, 20 + CALM + 1, 1.5);
  assert.equal(s.margin, MARGIN_MAX - MARGIN_STEP, 'one step after CALM s');
  steady(s, 20 + CALM + 1, 20 + 5 * CALM, 1.5);
  assert.equal(s.margin, 0);
});

test('V111-38 a step that was used stays: no step off while the least reading came close', () => {
  const s = new Steer();
  for (let i = 0; i < 2; i++) s.dry(i * BUMP_GAP);
  // Late segments drain it to 0.2 s now and then: not near dry, but a step less would have been.
  for (let t = 10; t < 10 + 3 * CALM; t += 0.25) s.step(t, Math.floor(t) % 30 === 0 ? 0.2 : 1.2);
  assert.equal(s.margin, 2 * MARGIN_STEP);
  steady(s, 10 + 3 * CALM, 10 + 4 * CALM + 1, 1.2);
  assert.equal(s.margin, MARGIN_STEP, 'a clean stretch takes it off');
});

test('V111-38 a dry run starts the calm count again', () => {
  const s = new Steer();
  s.dry(0);
  steady(s, 1, 1 + CALM - 10, 1);
  s.dry(1 + CALM - 10);
  steady(s, 1 + CALM - 10, 1 + 2 * CALM - 20, 1);
  assert.equal(s.margin, 2 * MARGIN_STEP, 'neither stretch was CALM s long');
});

test('V111-38 time not steering is not calm: pauses and gaps keep the margin', () => {
  const s = new Steer();
  s.dry(0);
  s.step(1, 1);
  s.step(1000, 1);
  s.reset();
  s.step(5000, 1);
  assert.equal(s.margin, MARGIN_STEP);
});

test('V111-38 no fixed spare by level: a stream on time keeps the small band at any quality', () => {
  const s = new Steer();
  assert.equal(s.extra, 0);
  assert.equal(s.keep, SPARE_KEEP);
  assert.equal(feed(s, 0, 6, 1.7), RATE_FAST, 'the spare V111-34 kept at 1080p is spare again');
  const t = new Steer();
  assert.equal(steady(t, 0, 300, 0.6), 1);
  assert.equal(t.margin, 0, '5 min on time: no extra delay');
});

test('V111-38 the margin raises the bands like the dry-run margin did', () => {
  const s = new Steer();
  for (let i = 0; i < 4; i++) s.dry(i * BUMP_GAP);
  assert.equal(steady(s, 20, 26, 0.6), RATE_SLOW, '0.6 s is thin with 1 s earned');
  assert.equal(steady(s, 26, 40, 0.6 + MARGIN_MAX), 1);
});

test('V111-38 the Live press lands mid-band plus the margin, and steering leaves it there', () => {
  const s = new Steer();
  const aheads = [
    { at: 0, ahead: 3 },
    { at: 4, ahead: 3 },
  ];
  assert.ok(Math.abs(edgeJump(aheads, s.keep)! - (3 - SPARE_KEEP)) < 1e-9);
  assert.equal(steady(s, 0, 20, SPARE_KEEP), 1, 'no speed up or slow down right after');
  s.dry(30);
  assert.ok(Math.abs(edgeJump(aheads, s.keep)! - (3 - SPARE_KEEP - MARGIN_STEP)) < 1e-9, 'keeps what the stream earned');
});
