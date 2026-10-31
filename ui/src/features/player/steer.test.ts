import assert from 'node:assert/strict';
import { test } from 'node:test';

import { RATE_FAST, RATE_SLOW, Steer } from './steer.ts';

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
