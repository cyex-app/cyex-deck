import assert from 'node:assert/strict';
import { test } from 'node:test';

import { MOD, modClasses } from './emotes.ts';

test('modifier bits become classes', () => {
  assert.equal(modClasses(undefined), '');
  assert.equal(modClasses(MOD.flipX | MOD.rainbow), 'm-flipX m-rainbow');
  assert.equal(modClasses(MOD.hidden), '');
});
