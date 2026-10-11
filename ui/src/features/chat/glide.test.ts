import assert from 'node:assert/strict';
import { test } from 'node:test';

import { GLIDE_MS, glideMs, MIN_GLIDE_MS } from './glide.ts';

test('a calm chat slides the full length (V103-12)', () => {
  assert.equal(glideMs(24, 600, Infinity), GLIDE_MS);
  assert.equal(glideMs(24, 600, 5000), GLIDE_MS);
});

test('a slide never outlasts the gap between arrivals', () => {
  assert.equal(glideMs(24, 600, 100), 100);
  assert.equal(glideMs(24, 600, MIN_GLIDE_MS), MIN_GLIDE_MS);
});

test('fast chats skip it: lines come before a slide could show', () => {
  assert.equal(glideMs(24, 600, 50), 0);
  assert.equal(glideMs(24, 600, 0), 0);
});

test('nothing moved, or too much at once, just shows', () => {
  assert.equal(glideMs(0, 600, Infinity), 0);
  assert.equal(glideMs(0.4, 600, Infinity), 0);
  assert.equal(glideMs(301, 600, Infinity), 0);
  assert.equal(glideMs(300, 600, Infinity), GLIDE_MS);
  assert.equal(glideMs(10, 0, Infinity), 0);
  assert.equal(glideMs(Number.NaN, 600, Infinity), 0);
});
