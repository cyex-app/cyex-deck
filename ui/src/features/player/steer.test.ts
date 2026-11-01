import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MARGIN_MAX, MARGIN_STEP, RATE_FAST, RATE_SLOW, REST, Steer } from './steer.ts';

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

test('V108-LL6 the margin adds up to a cap, survives a reset and wears off over minutes', () => {
  const s = new Steer();
  for (let i = 0; i < 10; i++) s.dry(i);
  assert.equal(s.margin, MARGIN_MAX);
  s.reset();
  assert.equal(s.margin, MARGIN_MAX, 'a pause or a jump keeps it');
  s.step(600, 1);
  assert.ok(Math.abs(s.margin - (MARGIN_MAX - MARGIN_STEP)) < 0.02, `10 min wear one step off, got ${s.margin}`);
  s.step(5000, 1);
  assert.equal(s.margin, 0);
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
