import assert from 'node:assert/strict';
import { test } from 'node:test';

import { fold } from './rewoundFold.ts';

test('folds players of one channel', () => {
  assert.equal(fold([]), undefined);
  assert.deepEqual(fold([{ behind: 30, ended: true }]), { behind: 30, ended: true });
  assert.deepEqual(fold([{ behind: 30, ended: true }, { behind: 10, ended: false }]), { behind: 10, ended: false });
  assert.deepEqual(fold([{ behind: 30, ended: true }, { behind: 10, ended: true }]), { behind: 10, ended: true });
});
