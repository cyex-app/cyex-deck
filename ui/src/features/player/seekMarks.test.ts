import assert from 'node:assert/strict';
import { test } from 'node:test';

import { markAt } from './seekMarks.ts';

const mk = (id: number, t: number) => ({ id, t, text: `m${id}`, kind: 'spike' });

test('a pointer within a few pixels is on the mark', () => {
  const list = [mk(1, 100), mk(2, 500)];
  // 1000 s over 500 px: 2 s a pixel, so 6 px is 12 s.
  assert.equal(markAt(list, 108, 1000, 500)?.id, 1);
  assert.equal(markAt(list, 113, 1000, 500), null);
  assert.equal(markAt(list, 490, 1000, 500)?.id, 2);
});

test('the nearest of two wins; nothing to hit is null', () => {
  const list = [mk(1, 100), mk(2, 106)];
  assert.equal(markAt(list, 104, 1000, 500)?.id, 2);
  assert.equal(markAt([], 1, 1000, 500), null);
  assert.equal(markAt(list, 100, 0, 500), null);
  assert.equal(markAt(list, 100, 1000, 0), null);
});
