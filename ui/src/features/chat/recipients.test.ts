import assert from 'node:assert/strict';
import { test } from 'node:test';

import { foldRecipients } from './recipients.ts';

const names = (n: number) => Array.from({ length: n }, (_, i) => `u${i}`);

test('a short list shows whole', () => {
  assert.deepEqual(foldRecipients(names(10), false), { shown: names(10), hidden: 0 });
});

test('one name over the limit is not folded', () => {
  assert.equal(foldRecipients(names(11), false).hidden, 0);
});

test('a long list folds after the limit', () => {
  const r = foldRecipients(names(100), false);
  assert.equal(r.shown.length, 10);
  assert.equal(r.hidden, 90);
});

test('open shows everything', () => {
  assert.deepEqual(foldRecipients(names(100), true), { shown: names(100), hidden: 0 });
});
