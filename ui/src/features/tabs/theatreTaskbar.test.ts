import assert from 'node:assert/strict';
import { test } from 'node:test';

import { backIn, OFF, roomStep } from './theatreTaskbar.ts';

test('V110-B10 the bottom 2 px show the taskbar at once', () => {
  assert.equal(roomStep(OFF, 2, 0).on, true);
  assert.equal(roomStep(OFF, 3, 0).on, false);
  assert.equal(roomStep(OFF, 200, 0).on, false);
});

test('V110-B10 it stays while the pointer is near the bottom or over the taskbar', () => {
  const on = roomStep(OFF, 1, 0);
  assert.equal(roomStep(on, 80, 5000).on, true);
  assert.equal(roomStep(on, null, 5000).on, true);
  assert.equal(roomStep(on, null, 5000).since, null);
});

test('V110-B10 covers the screen after 400 ms more than 80 px above the bottom', () => {
  let r = roomStep(OFF, 1, 0);
  r = roomStep(r, 81, 100);
  assert.deepEqual(r, { on: true, since: 100 });
  assert.equal(backIn(r, 300), 200);
  r = roomStep(r, 120, 499);
  assert.equal(r.on, true);
  r = roomStep(r, 120, 500);
  assert.equal(r.on, false);
});

test('V110-B10 dipping back near the bottom restarts the 400 ms', () => {
  let r = roomStep(OFF, 1, 0);
  r = roomStep(r, 200, 100);
  r = roomStep(r, 50, 300);
  assert.equal(r.since, null);
  r = roomStep(r, 200, 350);
  r = roomStep(r, 200, 700);
  assert.equal(r.on, true);
  assert.equal(roomStep(r, 200, 750).on, false);
});
