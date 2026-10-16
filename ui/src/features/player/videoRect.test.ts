import assert from 'node:assert/strict';
import { test } from 'node:test';

import { containRect } from './videoRect.ts';

test('bars at the sides when the box is wider', () => {
  const r = containRect(1000, 500, 800, 600);
  assert.ok(Math.abs(r.left - 166.6667) < 0.001);
  assert.equal(r.top, 0);
  assert.equal(r.height, 500);
});

test('bars above and below when the box is taller', () => {
  assert.deepEqual(containRect(400, 600, 1600, 900), { left: 0, top: 187.5, width: 400, height: 225 });
});

test('uses the whole box while the size is unknown', () => {
  assert.deepEqual(containRect(300, 200, 0, 0), { left: 0, top: 0, width: 300, height: 200 });
});
