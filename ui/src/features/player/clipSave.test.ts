import assert from 'node:assert/strict';
import { test } from 'node:test';

import { savePercent, saveText } from './clipSave.ts';

test('a percent only with a size', () => {
  assert.equal(savePercent({ done: 50, total: 200 }), 25);
  assert.equal(savePercent({ done: 999, total: 1000 }), 99);
  assert.equal(savePercent({ done: 5, total: null }), null);
  assert.equal(savePercent({ done: 5, total: 0 }), null);
  assert.equal(savePercent({ done: 300, total: 200 }), 100);
});

test('the words follow what is known', () => {
  assert.equal(saveText(undefined), 'Saving…');
  assert.equal(saveText({ done: 0, total: 100 }), 'Saving…');
  assert.equal(saveText({ done: 42, total: 100 }), 'Saving 42 %');
  assert.equal(saveText({ done: 3 * 1024 * 1024, total: null }), 'Saving 3 MB');
});
