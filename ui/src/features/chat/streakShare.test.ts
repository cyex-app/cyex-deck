import assert from 'node:assert/strict';
import { test } from 'node:test';

import { streakText } from './streakShare.ts';

test('V108-N23 prompt words', () => {
  assert.equal(streakText(5), '5-stream watch streak. Share it?');
  assert.equal(streakText(1234, (n) => '1.234'), '1.234-stream watch streak. Share it?');
  assert.equal(streakText(0), 'Share your watch streak?');
});
