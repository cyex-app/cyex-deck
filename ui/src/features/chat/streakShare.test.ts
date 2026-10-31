import assert from 'node:assert/strict';
import { test } from 'node:test';

import { streakShort, streakText } from './streakShare.ts';

test('V108-N23 row words', () => {
  assert.equal(streakText(5), '5-stream watch streak');
  assert.equal(streakText(1234, () => '1.234'), '1.234-stream watch streak');
  assert.equal(streakText(0), 'Watch streak');
  assert.equal(streakShort(5), '5-stream streak');
  assert.equal(streakShort(1234, () => '1.234'), '1.234-stream streak');
  assert.equal(streakShort(0), 'Streak');
});
