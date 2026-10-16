import assert from 'node:assert/strict';
import { test } from 'node:test';

import { activeIndex } from './activeCell.ts';

const keys = ['__none', 'a', 'b', 'c'];

test('starts on the worn one, else None', () => {
  assert.equal(activeIndex(keys, undefined, 'b'), 2);
  assert.equal(activeIndex(keys, undefined, '__none'), 0);
  assert.equal(activeIndex(keys, undefined, 'gone'), 0);
});

test('follows the moved id when the list changes', () => {
  assert.equal(activeIndex(keys, 'c', 'a'), 3);
  assert.equal(activeIndex(['__none', 'c'], 'c', 'a'), 1);
});

test('a moved id that was filtered out falls back to the worn one', () => {
  assert.equal(activeIndex(['__none', 'a'], 'c', 'a'), 1);
  assert.equal(activeIndex(['__none'], 'c', 'a'), 0);
});

test('the cursor and the worn cell are separate things', () => {
  // After Undo the cursor can sit on "a" while None is worn.
  assert.equal(activeIndex(keys, 'a', '__none'), 1);
  assert.equal(keys[activeIndex(keys, 'a', '__none')] === '__none', false);
});
