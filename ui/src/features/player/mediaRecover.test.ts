import assert from 'node:assert/strict';
import { test } from 'node:test';

import { RECOVER_WINDOW_MS, recoverMedia } from './mediaRecover.ts';

test('V110-F83 two recoveries in a row, the third fails', () => {
  const a = recoverMedia([], 1000)!;
  const b = recoverMedia(a, 2000)!;
  assert.equal(recoverMedia(b, 3000), null);
});

test('V110-F83 old recoveries stop counting after the window', () => {
  const b = recoverMedia(recoverMedia([], 1000)!, 2000)!;
  const later = 2000 + RECOVER_WINDOW_MS;
  assert.deepEqual(recoverMedia(b, later), [later]);
});
