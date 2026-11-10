import assert from 'node:assert/strict';
import { test } from 'node:test';

import { slideDeltas } from './slide.ts';

test('V110-A-idea tabs after a closed one travel back from where they were', () => {
  const before = new Map([['a', { x: 0, y: 0 }], ['b', { x: 100, y: 0 }], ['c', { x: 200, y: 0 }]]);
  const after = new Map([['a', { x: 0, y: 0 }], ['c', { x: 100, y: 0 }]]);
  assert.deepEqual([...slideDeltas(before, after)], [['c', { x: 100, y: 0 }]]);
});

test('V110-A-idea a tab that was not there before, or moved less than half a pixel, does not slide; rows work too', () => {
  const before = new Map([['a', { x: 0, y: 30 }], ['b', { x: 0.2, y: 0 }]]);
  const after = new Map([['a', { x: 0, y: 0 }], ['b', { x: 0, y: 0 }], ['new', { x: 5, y: 5 }]]);
  assert.deepEqual([...slideDeltas(before, after)], [['a', { x: 0, y: 30 }]]);
});
