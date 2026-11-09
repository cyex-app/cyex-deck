import assert from 'node:assert/strict';
import { test } from 'node:test';

import { dropPercent, dropText, withStats } from './statsPref.ts';

test('the dropped share is a percentage of the frames shown', () => {
  assert.equal(dropPercent(0, 0), 0);
  assert.equal(dropPercent(12, 3000), 0.4);
  assert.equal(dropPercent(5, 2), 100);
});

test('the dropped text has a comma decimal, a space before the sign and grouped counts', () => {
  assert.equal(dropText({ dropped: 12, total: 3000 }), '0,4 % (12)');
  assert.equal(dropText({ dropped: 0, total: 0 }), '0,0 % (0)');
  assert.equal(dropText({ dropped: 1500, total: 100000 }), '1,5 % (1.500)');
  assert.equal(dropText(null), '');
});

test('the tiles with stats on: added once, removed, capped', () => {
  assert.deepEqual(withStats(['a'], 'b', true), ['a', 'b']);
  assert.deepEqual(withStats(['a', 'b'], 'a', true), ['b', 'a']);
  assert.deepEqual(withStats(['a', 'b'], 'a', false), ['b']);
  const many = Array.from({ length: 250 }, (_, i) => `t${i}`);
  assert.equal(withStats(many, 'x', true).length, 200);
});
