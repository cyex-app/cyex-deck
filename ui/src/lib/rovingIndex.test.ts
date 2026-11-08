import assert from 'node:assert/strict';
import { test } from 'node:test';

import { rovingIndex } from './rovingIndex.ts';

test('V110-F32 rovingIndex: Up, Down, Home and End, clamped at the ends', () => {
  assert.equal(rovingIndex('ArrowDown', 0, 3), 1);
  assert.equal(rovingIndex('ArrowDown', 2, 3), 2);
  assert.equal(rovingIndex('ArrowUp', 0, 3), 0);
  assert.equal(rovingIndex('ArrowUp', 2, 3), 1);
  assert.equal(rovingIndex('Home', 2, 3), 0);
  assert.equal(rovingIndex('End', 0, 3), 2);
  assert.equal(rovingIndex('a', 0, 3), null);
  assert.equal(rovingIndex('End', 0, 0), null);
});
