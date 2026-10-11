import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MOD, emoteRatio, modClasses } from './emotes.ts';

test('modifier bits become classes', () => {
  assert.equal(modClasses(undefined), '');
  assert.equal(modClasses(MOD.flipX | MOD.rainbow), 'm-flipX m-rainbow');
  assert.equal(modClasses(MOD.hidden), '');
});

test('emotes keep their own shape (V102-06)', () => {
  assert.equal(emoteRatio(undefined), 'auto 1 / 1');
  assert.equal(emoteRatio([112, 32]), 'auto 112 / 32');
});
