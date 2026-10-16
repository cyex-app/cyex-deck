import assert from 'node:assert/strict';
import { test } from 'node:test';

import { accountChoices } from './accountChoices.ts';

test('sorted by display name, ignoring case', () => {
  const out = accountChoices([
    { userId: '3', displayName: 'zed' },
    { userId: '1', displayName: 'Bob', avatar: 'b.png' },
    { userId: '2', displayName: 'alice' },
  ]);
  assert.deepEqual(out.map((c) => c.label), ['alice', 'Bob', 'zed']);
  assert.equal(out[1].picture, 'b.png');
  assert.equal(out[0].picture, null);
});
